use super::{
    driver::ShellDriver,
    engine::{Action, PackageAction, Review, Workspace},
    history::now_ms,
    types::*,
};
use crate::backend::PocError;
use serde::Serialize;
use std::{
    collections::{BTreeSet, HashMap},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
type Result<T> = std::result::Result<T, PocError>;
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub package: String,
    pub review: Review,
}
#[derive(Debug, Clone, Serialize)]
pub struct Exclusion {
    pub package: String,
    pub reason: String,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkReview {
    pub id: String,
    pub target: Target,
    pub items: Vec<Item>,
    pub excluded: Vec<Exclusion>,
    pub expires_ms: i64,
}
struct Job {
    review: Option<BulkReview>,
    cancel: Arc<AtomicBool>,
    expires_ms: i64,
}
#[derive(Default)]
pub struct Bulk {
    jobs: Mutex<HashMap<String, Job>>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Outcome {
    package: String,
    status: OperationStatus,
    operation_id: Option<String>,
    message: Option<String>,
}
fn error(e: impl std::fmt::Display) -> PocError {
    PocError::new("bulk", e.to_string())
}
impl Bulk {
    pub async fn prepare(
        &self,
        w: &Workspace,
        d: &mut dyn ShellDriver,
        target: &Target,
        packages: Vec<String>,
        action: PackageAction,
    ) -> Result<BulkReview> {
        validate_target(d.current_target().as_ref(), target)?;
        if packages.is_empty() || packages.len() > 32 {
            return Err(PocError::new("input", "Select from 1 to 32 applications."));
        }
        {
            let mut jobs = self.jobs.lock().map_err(error)?;
            jobs.retain(|_, job| {
                if job.review.is_some() && job.expires_ms <= now_ms() {
                    if let Some(r) = &job.review {
                        for i in &r.items {
                            let _ = w.discard(&i.review.id);
                        }
                    }
                    false
                } else {
                    true
                }
            });
            if jobs.len() >= 8 {
                return Err(PocError::new(
                    "busy",
                    "Finish or dismiss an existing bulk review first.",
                ));
            }
        }
        let mut seen = BTreeSet::new();
        let mut items = Vec::new();
        let mut excluded = Vec::new();
        for package in packages {
            if !seen.insert(package.clone()) {
                continue;
            }
            match w
                .prepare(
                    d,
                    target,
                    Action::Package {
                        package: package.clone(),
                        action: action.clone(),
                    },
                )
                .await
            {
                Ok(review) => items.push(Item { package, review }),
                Err(e) => {
                    if validate_target(d.current_target().as_ref(), target).is_err() {
                        for item in &items {
                            let _ = w.discard(&item.review.id);
                        }
                        return Err(e);
                    }
                    excluded.push(Exclusion {
                        package,
                        reason: e.message,
                    });
                }
            }
        }
        let review = BulkReview {
            id: uuid::Uuid::new_v4().to_string(),
            target: target.clone(),
            items,
            excluded,
            expires_ms: now_ms() + 600_000,
        };
        self.jobs.lock().map_err(error)?.insert(
            review.id.clone(),
            Job {
                review: Some(review.clone()),
                cancel: Arc::new(AtomicBool::new(false)),
                expires_ms: review.expires_ms,
            },
        );
        Ok(review)
    }
    pub fn cancel(&self, id: &str) -> Result<()> {
        let jobs = self.jobs.lock().map_err(error)?;
        let job = jobs
            .get(id)
            .ok_or_else(|| error("This bulk action is no longer active."))?;
        job.cancel.store(true, Ordering::SeqCst);
        Ok(())
    }
    pub fn discard(&self, w: &Workspace, id: &str) -> Result<()> {
        let mut jobs = self.jobs.lock().map_err(error)?;
        if jobs.get(id).is_some_and(|j| j.review.is_none()) {
            return Err(error(
                "This bulk action is already running. Cancel its remaining items instead.",
            ));
        }
        if let Some(job) = jobs.remove(id)
            && let Some(review) = job.review
        {
            for item in review.items {
                w.discard(&item.review.id)?;
            }
        }
        Ok(())
    }
    pub async fn apply(
        &self,
        w: &Workspace,
        d: &mut dyn ShellDriver,
        id: &str,
        stop_after_failure: bool,
    ) -> Result<OperationRecord> {
        let (review, cancel) = {
            let mut jobs = self.jobs.lock().map_err(error)?;
            let job = jobs
                .get_mut(id)
                .ok_or_else(|| error("This bulk review expired or was dismissed."))?;
            let review = job
                .review
                .take()
                .ok_or_else(|| error("This bulk review was already used."))?;
            (review, job.cancel.clone())
        };
        let result = self.run(w, d, &review, cancel, stop_after_failure).await;
        for item in &review.items {
            let _ = w.discard(&item.review.id);
        }
        self.jobs.lock().map_err(error)?.remove(id);
        result
    }
    async fn run(
        &self,
        w: &Workspace,
        d: &mut dyn ShellDriver,
        review: &BulkReview,
        cancel: Arc<AtomicBool>,
        stop_after_failure: bool,
    ) -> Result<OperationRecord> {
        validate_target(d.current_target().as_ref(), &review.target)?;
        if now_ms() > review.expires_ms {
            return Err(error(
                "This bulk review expired. Review the current state again.",
            ));
        }
        if review.items.is_empty() {
            return Err(error("No eligible changes were included in this review."));
        }
        let now = now_ms();
        let mut row = OperationRecord {
            id: uuid::Uuid::new_v4().to_string(),
            target: review.target.clone(),
            title: format!("Bulk app changes · {} apps", review.items.len()),
            device_label: d.target_label(),
            kind: "bulkPackages".into(),
            status: OperationStatus::Running,
            created_ms: now,
            updated_ms: now,
            output: String::new(),
            error: None,
            recovery: None,
        };
        w.history.put(&row)?;
        let mut outcomes = Vec::new();
        let mut stopped = false;
        for item in &review.items {
            if stopped || cancel.load(Ordering::SeqCst) {
                outcomes.push(Outcome {
                    package: item.package.clone(),
                    status: OperationStatus::Cancelled,
                    operation_id: None,
                    message: Some(
                        "Not dispatched. Previously completed items stay changed.".into(),
                    ),
                });
                continue;
            }
            let result = w.apply(d, &item.review.id).await;
            let outcome = match result {
                Ok(r) => Outcome {
                    package: item.package.clone(),
                    status: r.status,
                    operation_id: Some(r.id),
                    message: r.error,
                },
                Err(e) => Outcome {
                    package: item.package.clone(),
                    status: OperationStatus::Failed,
                    operation_id: None,
                    message: Some(e.message),
                },
            };
            let failure = matches!(
                outcome.status,
                OperationStatus::Failed | OperationStatus::Partial | OperationStatus::Unknown
            );
            if failure
                && (stop_after_failure
                    || validate_target(d.current_target().as_ref(), &review.target).is_err())
            {
                stopped = true;
            }
            outcomes.push(outcome);
            row.output = serde_json::to_string(
                &serde_json::json!({"items":outcomes,"excluded":review.excluded}),
            )
            .map_err(error)?;
            row.updated_ms = now_ms();
            w.history.put(&row)?;
        }
        let verified = outcomes
            .iter()
            .filter(|o| o.status == OperationStatus::Verified)
            .count();
        let sent = outcomes
            .iter()
            .filter(|o| o.status == OperationStatus::Sent)
            .count();
        let unknown = outcomes
            .iter()
            .any(|o| o.status == OperationStatus::Unknown);
        let cancelled = outcomes
            .iter()
            .filter(|o| o.status == OperationStatus::Cancelled)
            .count();
        row.status = if verified == outcomes.len() && review.excluded.is_empty() {
            OperationStatus::Verified
        } else if cancelled == outcomes.len() {
            OperationStatus::Cancelled
        } else if sent == outcomes.len() && review.excluded.is_empty() {
            OperationStatus::Sent
        } else if verified + sent > 0 {
            OperationStatus::Partial
        } else if unknown {
            OperationStatus::Unknown
        } else {
            OperationStatus::Failed
        };
        row.output = serde_json::to_string(
            &serde_json::json!({"items":outcomes,"excluded":review.excluded}),
        )
        .map_err(error)?;
        row.updated_ms = now_ms();
        w.history.put(&row)?;
        Ok(row)
    }
}
