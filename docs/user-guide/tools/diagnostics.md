# Find out why something isn't working

<p class="lead">Read what the TV is saying about itself. Everything here only reads, so it's safe to explore.</p>

Open **Tools → Diagnostics** and choose a section.

| Section | Use it to |
|---|---|
| **Recent logs** | See errors, from one app or the whole TV |
| **Processes** | See everything running |
| **Memory** | See what's using memory |
| **Services** | See what's running in the background |
| **Display** | See the resolution and display modes |
| **Input devices** | See every remote and keyboard, and which keys each has |
| **Battery** | See what the TV reports |

!!! screenshot "diagnostics-logs.png"
    Diagnostics on Recent logs, with the Show, From and Newest choices.

## Read an app's logs

1. Open the app on the TV and make the problem happen.
2. Under **Show**, keep **Warnings and errors**, or pick **Only errors**, **Normal detail** or **Everything**.
3. Under **From**, choose the app, or keep **The whole TV**.
4. Under **Newest**, choose **200 lines**, **1000 lines** or **5000 lines**.
5. Tap **Show logs**. **Save log** saves `tv-log.txt` to your phone.

"That app isn't running. Open it on the TV, then try again." means Bridgecoil needs the running app to filter by it.

Logs can include things apps wrote to them, so read one through before you share it.

For specific questions, such as why the TV last restarted, see the Logs group in [Commands](commands.md).
