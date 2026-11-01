# How a change is reviewed and undone

<p class="lead">Nothing that changes your TV runs without you seeing it first.</p>

## The review

A review shows:

- The change, and which TV and profile it's for
- The current state
- What it does
- A warning box for anything you could lose
- Whether it can be undone
- On the **Commands** tab, the exact command that will run

Tick **Apply this change to the reviewed target**, then tap **Apply change**. The box starts unticked every time. To cancel, close the review.

!!! screenshot "review-effect.png"
    Review change on the Effect tab.

!!! screenshot "review-commands.png"
    Review change on the Commands tab, showing the exact command.

## Why a review can stop working

A review belongs to one TV, connection and profile, and lasts 10 minutes. Bridgecoil reads the state again just before applying. If it has changed, the change stops, so a stale review can't do something you didn't intend.

| Message | What to do |
|---|---|
| "This review expired. Inspect the current state again." | Start the action again |
| "The TV's state changed after review. Review the action again." | Something else changed the TV. Check and start again |
| "The device, connection or profile changed. Review this action again." | Reconnect and start again |

After a change, **Operation result** shows the outcome, and **View history** opens its record.

## What the results mean

| Result | Meaning |
|---|---|
| **Verified** | Bridgecoil read the expected state back from the TV |
| **Sent** | The command ran, and its effect couldn't be checked |
| **Partial** | Part of a bulk change worked |
| **Unknown** | Bridgecoil couldn't tell. Check the TV before trying again |
| **Failed** | It didn't work |
| **Cancelled** | It was stopped before running |

If Bridgecoil closes during a change, the record becomes Unknown. Bridgecoil never replays an interrupted change.
