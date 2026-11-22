# Testing Sub20 by hand

Thanks for trying Sub20 before 1.0. This sheet takes about twenty minutes
for parts 1 to 3; the rest is for whoever has the hardware or the time.
Part 9 is Sub20 Lite, in a browser, and can be done on its own, on a
phone. Do the steps in order, and note any step where what happened isn't
what the sheet says.

## Before you start

- **Which download:** on the [releases page](https://github.com/Y4dd/Sub20/releases/latest),
  take **Sub20** for your system (the table at the top of the release
  says which file). Take **Sub20 without FFmpeg** only if part 8 asks you
  to.
- **A few clips:** two or three videos you would really send, at least one
  over 20 MB. A game clip from the NVIDIA app, OBS or Xbox Game Bar, and a
  phone video, are the most useful.
- **Discord:** a channel or a DM where you can paste a test clip.

### How to report

1. In Sub20, open **Settings** (in the title bar) and press
   **Copy diagnostics** at the bottom. It copies the versions, your
   system, GPU and encoders, and the FFmpeg log of the video on screen.
   Your home folder is written as `~`, and nothing is sent anywhere.
2. Open a [bug report](https://github.com/Y4dd/Sub20/issues/new/choose),
   say which step of this sheet it was, what you expected and what
   happened, and paste the diagnostics into its box.
3. Things that went right are worth telling too, with your system and GPU:
   one line in an issue or a message to the owner is enough.

## 1. Install and first launch

1. **Windows:** run the installer. If SmartScreen says "Windows protected
   your PC", click **More info**, then **Run anyway**.
   *Expected:* it installs and Sub20 opens.
2. **Linux:** install the deb or the rpm, or make the AppImage executable
   (`chmod +x Sub20*.AppImage`) and run it.
   *Expected:* Sub20 is in your applications menu (deb, rpm) and opens.
3. *Expected on both systems:* the first screen shows the yellow bracket
   and **Drop videos here**. Settings lists encoders under **Encoders**,
   each marked **Works** or **Not working**.

## 2. Compress and send to Discord

1. Drop two or three clips on the window, or press **Choose videos**.
   *Expected:* they appear in the list on the left with a filmstrip, and
   the first one plays in the preview.
2. Drag the yellow handles under the preview to keep a few seconds.
   *Expected:* the line under the timeline says what will come out (size,
   resolution and how it should look) and changes as you drag.
3. Leave the size at 20 MB and press **Compress** (Ctrl+Enter).
   *Expected:* the kept part of the bracket fills from the left, with a
   percentage and the time left, and "Compressing on the NVIDIA GPU" (or
   your card's maker; "CPU" when no card can do it).
4. When a clip is done, switch between **Original** and **Result**.
   *Expected:* the result plays, you can click anywhere on its filmstrip to
   jump there, and its size is under 20 MB.
5. On **Result**, drag the yellow handles in a little, then press **Cut**.
   *Expected:* the line under the filmstrip says how long and about how big
   the cut will be; after **Cut**, the result is shorter and smaller, and
   plays from where the start handle sat.
6. Press **Copy** (or Ctrl+C), then paste into Discord with Ctrl+V.
   *Expected:* Discord attaches the video, and it plays in the chat after
   sending.
7. Drag the file chip (the file's name under the timeline) into a Discord
   channel, and once into a folder.
   *Expected:* the video is attached, and the folder gets a copy.
8. Rename a finished clip: double-click its name in the list (or F2),
   type a new name and press Enter.
   *Expected:* the file on disk has the new name; **Show in folder** finds
   it.
9. Drop a folder that has videos in it and in folders inside it, or use
   **Or choose a folder of videos** on the first screen.
   *Expected:* the list fills in by name as the videos are read, with
   Sub20's own results left out.
10. Press **Compress** on several clips at once, then **Cancel** one of them
   while it runs.
   *Expected:* the canceled one stops and says **Canceled**; the others
   finish.
11. In Settings, under **Where results go**, choose **Replace the original**,
   then compress a copy of a clip (not one you want to keep).
   *Expected:* the original goes to the recycle bin or trash, and the result
   takes its name.

## 3. The window

1. **Windows:** drag the window by its title bar, snap it to a side of the
   screen, maximise and restore it with the title bar's buttons, and move
   it to a second monitor if you have one.
   *Expected:* each works like any other window.
2. While a clip is compressing, close the window (the close button or
   Alt+F4).
   *Expected:* a question offers **Stop and quit** or **Keep compressing**;
   **Keep compressing** keeps it going, **Stop and quit** closes Sub20 and
   leaves no half-written file beside the clip.
3. Press `?`.
   *Expected:* the keyboard shortcuts open; Escape closes them.
4. **Linux:** the preview plays the original and the result.

## 4. The folder watch (Windows)

1. In Settings, under **Watch a folder**, press **Watch NVIDIA app clips**
   or **Watch Xbox Game Bar clips** (each appears when its folder exists).
   *Expected:* the title bar says **Watching**.
2. Save a clip with the NVIDIA app or Game Bar while Sub20 is open.
   *Expected:* it joins the list by itself, ready to trim, and the video
   you had selected stays selected.
3. Switch on **Compress new videos straight away**, then save another clip.
   *Expected:* it is compressed by itself; **History** (in the title bar) lists it with **Open** and **Copy**, and a notice says it is
   ready. Click the notice while it is on screen: Sub20 comes forward with
   the clip finished, ready to copy.
4. Still with that on, save two clips while a game fills the screen, both
   full screen and borderless if you can, then leave the game.
   *Expected:* no notice during the game; one notice afterwards saying how
   many are ready.
5. Switch **Compress new videos straight away** off again and **Keep
   watching when the window is closed** on, then close the window.
   *Expected:* Sub20's icon stays in the tray. Save a clip. *Expected:* a
   notice says it is on the bench. Click it while it is on screen.
   *Expected:* the window comes back with that clip selected.
6. Right-click the tray icon and choose **Pause watching**, then save a
   clip. *Expected:* the title bar says **Watch paused** and the clip
   doesn't arrive. **Resume watching** brings it in.
7. Switch **Compress new videos straight away** on, close the window,
   choose **Pause watching** in the tray, and save six clips (or copy six
   videos into the folder). Then choose **Resume watching**. *Expected:* a
   notice says 6 clips arrived while Sub20 wasn't watching. Click it: the
   window opens and asks whether to compress or skip them.
8. When an update is out, with the window closed: right-click the tray
   icon. *Expected:* **Update to x.y.z** is in the menu. Choose it.
   *Expected:* the installer runs, and Sub20 comes back in the tray with
   no window, as the new version (its About line in Settings says so).
9. Switch on **Start in the tray when I sign in**, then sign out of
   Windows and back in. *Expected:* Sub20's icon is in the tray with no
   window. Click the icon. *Expected:* the window opens and its title bar
   says **Watching**.
10. Right-click the tray icon and choose **Quit**. *Expected:* Sub20 closes
    and the icon goes.

## 5. Opening videos from the system

1. **Windows:** right-click a video in Explorer, **Open with**, Sub20.
   Then select several videos, right-click, and choose **Compress with
   Sub20** (under **Show more options** on Windows 11).
   *Expected:* they all arrive in Sub20's list together, in name order.
2. With Sub20 already open, open another video with it the same way.
   *Expected:* it joins the list in the open window; no second window.
3. **Linux:** open a video with Sub20 from your file manager.
   *Expected:* they arrive in the list.

## 6. Updating

When the next version is out (the owner will say):

1. Open Sub20. *Expected:* Settings, under **Updates**, says it is out,
   with **Update and restart**.
2. Press it. *Expected:* Sub20 restarts as the new version, and stays the
   kind you installed (with or without FFmpeg; a deb stays a deb, an rpm
   an rpm).

## 7. A screen reader

With NVDA on Windows, or Orca on Linux:

1. Move through the window with Tab. *Expected:* every button and field is
   reached and has a name that says what it does.
2. Open Settings, Options and History. *Expected:* focus stays inside the
   open sheet; Escape closes it and focus goes back where it was.
3. Change the size and the options' choices with the arrow keys.
   *Expected:* each change is announced.

## 8. Hardware and special cases

Only if you have them:

- **An Intel or AMD graphics card:** compress a clip (part 2, steps 1 to
  4). *Expected:* it says "Compressing on the Intel GPU" or "on the AMD GPU",
  and the result is under the size. Send the diagnostics either way.
- **Linux:** the same, "on the GPU (VAAPI)" on an Intel or AMD card,
  where the driver has it.
- **An HDR video** (an iPhone on its default settings, or a game recorded
  in HDR). *Expected:* the line under the timeline says "HDR, converted to
  ordinary colour", and the result's colours look like the original, not
  grey.
- **A video with the microphone on its own track** (OBS or the NVIDIA app
  set that way). *Expected:* the line says "2 sound tracks, mixed", and
  your voice is in the result.
- **Sub20 without FFmpeg, on a computer with no FFmpeg.** *Expected:* the
  first screen says **FFmpeg is needed first** and offers **Download
  FFmpeg**; after it downloads, a clip compresses as in part 2.
- **If Sub20 ever closes by itself:** open it again, press **Copy
  diagnostics** (it includes the crash log), and report it.
- **Nitro:** if you have it, the largest file Discord takes at 50 MB,
  500 MB and 1 GB tells us how its limits are counted. Try a result a
  little under each.

## 9. Sub20 Lite, in a browser

Sub20 Lite is the same app in a web page, encoding on your browser's own
graphics card, with nothing uploaded. The owner will send its address once
it is online. This part works on a phone as well as a computer and takes
about ten minutes; what matters most is whether every result stays under
the size.

1. Open the address in the browser you normally use: Chrome, Edge, Firefox
   or Safari, on a computer, an Android phone or an iPhone.
   *Expected:* the yellow bracket, saying **Choose videos** on a phone or
   **Drop videos here** on a computer. If it says **This browser can't do
   it** instead, report that with the browser's name and version, and stop
   here.
2. Tap the bracket (or drop videos on it) and choose up to five videos you
   would really send, at least one over 20 MB. Phone videos and game clips
   are the most useful.
   *Expected:* each appears in the list with a filmstrip.
3. Leave the size at 20 MB and press **Compress**.
   *Expected:* each finishes and shows its size under the filmstrip.
4. Write down, for each video: its name or what it was, its length, the
   size before, the size Lite shows after, and roughly how long it took.
   *Expected:* every size is under 20 MB. One that isn't is the most
   important thing to report.
5. Send one result to Discord: **Share** on a phone, then Discord;
   **Download** on a computer, then drag the file into a channel.
   *Expected:* it uploads, and it plays in the chat on your phone and on
   a computer.
6. Report the list from step 4 and whether step 5 played, with the
   browser, its version, the device, and its graphics card if you know
   it. There is no Copy diagnostics in Lite, so those few words are what
   tell us where it ran.
