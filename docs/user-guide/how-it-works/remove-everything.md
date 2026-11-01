# Remove everything

<p class="lead">There's no single uninstall button yet, so these are the steps. Together they remove all of Bridgecoil's traces from your TV.</p>

Do them in this order, while connected.

## 1. Remove your mappings

In **Tools → Buttons**, tap **Remove all**. This stops the listener and deletes everything in `/data/local/tmp/bridgecoil`. It can't be undone.

Any app you turned off for a button stays off. Turn it back on in [Apps](../apps/turn-off-or-remove-an-app.md).

## 2. Remove the startup script

In **Tools → TV setup**, tap **Remove the startup script**. This removes only `00-bridgecoil`.

## 3. Remove Termux

In **Apps**, find **Termux**, then choose **Remove from profile**. Do the same for **Termux:Boot**. This deletes the agent, its key and anything else in Termux. If you use Termux for other things, keep it and delete only the `.bridgecoil` folder.

## 4. Put your launcher back

If you changed launchers, use [Restore your TV's launcher](../home-screen/restore-your-launcher.md), and turn back on any apps you turned off in [Clean up](../clean-up/bring-apps-back.md).

## 5. Delete the temporary files

In **Tools → Shell**, review and run:

```sh
rm -f /data/local/tmp/bridgecoil-inspector-*.jar /data/local/tmp/bridgecoil-recording.mp4 /data/local/tmp/bridgecoil-use-helper
```

## 6. Remove your phone's access

On the TV, open **Wireless debugging** and remove the paired devices, including **Bridgecoil TV**, which is the agent. The option **Revoke USB debugging authorizations** in Developer options does this for legacy connections.

Then turn off **Wireless debugging**. If you don't want it any more, turn off **Developer options** too.

## 7. Remove the app

Uninstall Bridgecoil from your phone. This deletes its key, history and saved TVs.

## What's left

Settings you changed yourself, such as animation speed, stay as you set them. Undo those from [History](../history/undo-a-change.md) before step 7, or change them back in the TV's settings.
