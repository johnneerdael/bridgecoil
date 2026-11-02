# Change many apps at once

<p class="lead">Select mode applies one action to up to 32 apps.</p>

1. On the Apps page, tap **Select**. Rows turn into checkboxes.
2. Tick the apps you want, or tap **Select shown** to select loaded apps up to the 32-app limit. Nothing is ticked automatically. **Clear selection** unticks them all.
3. Choose an action from the dropdown. Choose **Uninstall** to delete apps you added; **Disable** remains the default.
4. Tap **Review**. All included and excluded apps appear together; open any app's row to inspect its impact.
5. Tick the box to apply, then tap **Apply**.

![Apps in Select mode with several apps ticked.](../assets/shots/apps-bulk-select.png){ .bc-shot loading=lazy }

The review shows each app as **Included** or **Excluded · will not run**. An app is left out, with the reason, if it's rated Unsafe, it's protected, it's your launcher, or the TV didn't report its state. System apps are also excluded from **Uninstall**; use **Remove from profile** or **Disable** for those instead.

**Uninstall deletes each included app and its data from every profile on the TV. There is no automatic undo.** One confirmation applies to the whole batch; you do not need to confirm each app separately.

**Stop after the first failed or unknown item** is on by default. Leave it on, so a problem doesn't carry on through the list.

Changes apply one at a time, and those already done stay done if a later one fails. **Cancel remaining items** stops after the current app.

After a change, the list keeps its search, filter and browsing position. Apps successfully uninstalled disappear; apps still present remain selected. If refreshing fails, the existing list stays visible and **Retry refreshing apps** reloads it before you can continue loading more rows.

## Read the result

| Result | Meaning |
|---|---|
| **Done** | Checked on the TV |
| **Sent** | The command ran, and the effect couldn't be checked |
| **Not run** | It wasn't reached |
| **Failed** | It didn't work |
| **Partly done** | Some of it worked |
| **Unconfirmed** | Bridgecoil couldn't tell |
| **Left out** | It was excluded, with the reason |

Each app gets its own row in [History](../history/index.md). Supported inverse actions can be reviewed there; uninstall and deleted app data cannot be undone.
