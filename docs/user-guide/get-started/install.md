# Install Bridgecoil on your phone

<p class="lead">Bridgecoil runs on your phone or iPhone. You install nothing on the TV at this point.</p>

## Android

You need Android 7 or later.

1. Open the [latest release](https://github.com/johnneerdael/bridgecoil/releases/latest) on your phone.
2. Download the file ending in `arm64.apk`.
3. Open it and allow your browser to install apps if Android asks.

Bridgecoil isn't on Google Play, so Play Protect may warn you about it. Each release is signed with the same key, so later versions update the app you already have, and your saved TVs stay in place.

## iPhone and iPad

You need iOS 15 or later. Bridgecoil is available as a TestFlight beta. See the [download section of the website](https://johnneerdael.github.io/bridgecoil/#download) for how to join.

iOS asks for permission to use your local network the first time Bridgecoil looks for a TV. Allow it, because the app can't reach the TV without it. If you refused, turn it back on in **Settings → Privacy & Security → Local Network**.

iOS may close the connection when you leave the app, so you may have to reconnect when you come back.

## What Bridgecoil asks for on your phone

| Permission on Android | What it's used for |
|---|---|
| Internet and local network | Reaching your TV, and GitHub when you download ratings, tools or launchers |
| Network state | Finding TVs on your network |
| Foreground service | Keeping the connection open while the app is in the background. You see a **Connected to your TV** notification while this runs |

The app has no account, no advertising and no analytics. [What leaves your phone](../how-it-works/privacy.md) has the details.
