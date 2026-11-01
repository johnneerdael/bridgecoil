# Move files to and from your TV

<p class="lead">Browse the TV's storage, copy files either way, and rename, move or delete them.</p>

Open **Tools → Files**. The buttons at the top go to **Internal storage**, **Downloads**, **Movies**, **Music**, **Pictures** and **Device**. Device is the whole file system, most of which Android keeps private.

!!! screenshot "files-browse.png"
    Files in Internal storage, with folders and files.

Tap a folder to open it, and use the path at the top to go back. The parent icon goes up one level, and **Go to a path** lets you type one starting with `/`. Hidden items start with a dot, and **Show hidden** reveals them.

## Copy a file to the TV

Tap **Upload file**, choose a file, check **Name on the TV** and tap **Upload**. **Cancel upload** stops it. Files never overwrite one already there. The file is written under a temporary name, then gets its real name when it's complete, so a failed upload can't leave a broken file with the right name.

## Copy a file to your phone

Tap the actions icon on a file, then **Save to this device**, and choose where to save it. Folders can't be downloaded. If you cancel, delete any partial file left on your phone.

## Rename, move, delete and new folder

These go through a review.

| Action | How | Can you undo it? |
|---|---|---|
| **Rename** | Actions icon, new name, **Continue** | Yes, from History |
| **Move** | Actions icon, browse to the destination while the **Moving** bar shows, **Move here** | Yes, from History |
| **New folder** | **New folder** at the bottom, name, **Continue** | No, delete it by hand |
| **Delete** | Actions icon | No. The TV has no trash |

Deleting a folder deletes everything in it, and the review tells you how many items that is. Deleting a link removes only the link.

Bridgecoil refuses to delete or move folders that shared storage depends on, such as `/sdcard` and `/sdcard/Android`, with "This folder is protected, because shared storage depends on it."

## What you'll see

| Message | Meaning |
|---|---|
| "This folder is empty" or "Only hidden items here" | There's nothing to show, or only hidden items |
| "Couldn't load this folder" | Reading it failed. The error follows |
| "This is a file, not a folder" | The path you typed is a file. Choose the folder that contains it |
| "No access to this folder" | Android doesn't let ADB list it. Internal storage is usually open |
| "Folder not available" | It may not exist on this TV, or Android may be hiding it |
| "This folder is read-only for ADB." | You can look, and you can't change anything here |
| "Showing the first {n} of {total} items." | The list stops at 5,000 items |
| "The TV refused this file name or location." | Try a different folder. Names can't contain `" * : < > ? \ |` |
| "The upload stalled." | Nothing moved for 60 seconds. Check the connection |

Files needs no [TV setup](../get-started/set-up-your-tv.md).
