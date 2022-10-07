# s2yt

s2yt is a utility that automatically copies a YouTube URL to the clipboard for any Spotify track you play.
Whenever it detects a new track being played, it will look it up on YouTube and copy the URL for it to your clipboard automatically.

## Download

Grab the latest release [here](https://github.com/Raphiiko/s2yt/releases/tag/refs%2Fheads%2Fmain)

## Usage

The first time running s2yt, it will ask for a Spotify Client ID. To obtain this, you need to do the following:

1. Log in on https://developer.spotify.com/dashboard
2. Create a new application
3. Take the email address you registered your spotify account under, and add it as a user/tester
4. Edit the settings for the application, and add http://localhost:8888/callback under "Redirect URIs"
5. Save, copy the Client ID and give it to s2yt when it asks for it. 

Now you should be able to connect your Spotify account to s2yt (It will ask for it).

Once this has been done, s2yt should start doing its thing. With future launches it may occasionally ask to reauthenticate.

## FAQ

### I want to change my Spotify Client ID. How do I do this?
Your Client ID is stored in the config file at `%APPDATA%/s2yt/config/default-config.toml`. Delete this file, and s2yt will ask you for a new Client ID the next time it launches.
