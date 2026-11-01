# Frequently asked questions

## Do I need a computer?

No. Bridgecoil speaks ADB itself, from your phone.

## Does this void my warranty or root my TV?

Bridgecoil doesn't root anything, and it only uses Android's standard debugging feature. Warranty rules vary by brand, so check yours if that matters to you.

## Can it break my TV?

Removing or disabling the wrong system app can stop things working. Bridgecoil's review, ratings and protections make that harder, and most changes can be undone. [Shell](../tools/shell.md) and [Commands](../tools/commands.md) have fewer protections.

## Does Bridgecoil keep running on my phone?

No. After setup, your mappings run on the TV by themselves, with your phone off.

## Does it work on Fire TV or other Android devices?

It works with devices that offer ADB. It's built and tested for Android TV and Google TV, so Fire TV and other devices are untested.

## Can I control two TVs?

Yes, one at a time. Saved TVs keeps up to 32, and each TV needs its own setup.

## Does my remote's microphone or typing get recorded?

No. The listener only receives button events, and it writes down only what it acted on. See [How your buttons work](../how-it-works/how-buttons-work.md).

## Why does wireless debugging have to stay on?

The agent connects through it at each restart, so mappings can't come back without it.

## Why is Termux needed? Can't an app do this?

An installed app can't see your remote's button events. See [Why it needs this much access](../how-it-works/why-it-needs-access.md).

## Is Bridgecoil open source?

The source is public on [GitHub](https://github.com/johnneerdael/bridgecoil), so you can read what it does.

## How do I remove it?

See [Remove everything](../how-it-works/remove-everything.md).
