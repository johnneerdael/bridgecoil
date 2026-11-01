//! The small agent that runs in Termux. It finds the TV's own ADB and gets shell access from there,
//! which is what brings button mappings back after a restart, with no phone and no helper app.
use crate::shell_quote;
use sha2::{Digest, Sha256};
use std::sync::LazyLock;

const HOME: &str = "/data/data/com.termux/files/home";
pub const AGENT: &str = "/data/data/com.termux/files/home/.bridgecoil/agent";
const READY: &str = "/data/data/com.termux/files/home/.bridgecoil/agent-ready";

const ARM: &[u8] = include_bytes!("../../assets/agent/armeabi-v7a/bridgecoil-agent");
const ARM64: &[u8] = include_bytes!("../../assets/agent/arm64-v8a/bridgecoil-agent");
const X86_64: &[u8] = include_bytes!("../../assets/agent/x86_64/bridgecoil-agent");

/// The build for this TV. `abis` lists what it can run, best first. Termux uses the first one,
/// which is the one that matters.
pub fn binary_for(abis: &[String]) -> Option<&'static [u8]> {
    abis.iter().find_map(|abi| match abi.as_str() {
        "arm64-v8a" => Some(ARM64),
        "armeabi-v7a" => Some(ARM),
        "x86_64" => Some(X86_64),
        _ => None,
    })
}

static CURRENT: LazyLock<Vec<String>> = LazyLock::new(|| {
    [ARM, ARM64, X86_64]
        .iter()
        .map(|binary| format!("{:x}", Sha256::digest(binary)))
        .collect()
});

/// The SHA-256 of each agent build in this app.
pub fn current_sha256() -> &'static [String] {
    &CURRENT
}

/// Whether an agent on a TV is one this app ships. If not, it came with an older app.
pub fn is_current(sha256: &str) -> bool {
    CURRENT.iter().any(|known| known.eq_ignore_ascii_case(sha256))
}

/// Copies a pushed agent into Termux, where nothing else can reach it.
pub fn place_command(staged: &str) -> String {
    let staged = shell_quote(staged);
    let inner = shell_quote(&format!(
        "mkdir -p {HOME}/.bridgecoil && cat {staged} > {AGENT}.new && chmod 700 {AGENT}.new && mv {AGENT}.new {AGENT}"
    ));
    format!("run-as com.termux sh -c {inner}; result=$?; rm -f {staged}; exit $result")
}

fn agent_command(args: &str) -> String {
    format!("run-as com.termux env HOME={HOME} {AGENT} {args}")
}

/// Sets `port` to the open pairing screen's port, from Android's own record of what it advertises.
/// Android 14 can announce that port once and leave every later search unanswered, as a TCL did.
/// Apps ask `NsdManager`, which Termux can't reach, but this shell can read the same record.
/// Closed sessions stay in the record, so only a port that's listening counts.
const PAIRING_PORT: &str = r#"port=$(for found in $(dumpsys servicediscovery 2>/dev/null | sed -n 's/.*_adb-tls-pairing\._tcp.* port: \([0-9]*\).*/\1/p'); do
  if { ss -tln || netstat -tln; } 2>/dev/null | grep -q ":$found "; then echo "$found"; break; fi
done)"#;

/// Pairs with the TV's open pairing screen. With no port in Android's record, which is how Android
/// 11 to 13 behave, the agent is left to search and those TVs answer it.
pub fn pair_command(code: &str) -> Option<String> {
    (code.len() == 6 && code.bytes().all(|b| b.is_ascii_digit())).then(|| {
        format!(
            "{PAIRING_PORT}\n{}",
            agent_command(&format!("pair {code} $port"))
        )
    })
}

/// Proves the agent has shell access, and only then records it.
/// A TV on classic network ADB shows its approval prompt at this point.
pub fn verify_command() -> String {
    format!(
        "output=$({} 2>&1)\n\
         case \"$output\" in\n\
           *uid=2000\\(*) run-as com.termux touch {READY}; echo route=shell ;;\n\
           *) echo route=none; echo \"$output\" ;;\n\
         esac\n\
         exit 0",
        agent_command("run id")
    )
}
