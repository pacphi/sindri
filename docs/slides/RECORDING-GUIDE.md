# Recording Slide Decks with Music for YouTube

This guide covers how to record reveal.js slide decks with background music and "Now Playing" overlays for YouTube upload.

## Overview

| Tool           | Purpose                           | Cost |
| -------------- | --------------------------------- | ---- |
| **OBS Studio** | Screen recording with overlays    | Free |
| **Camtasia**   | Screen recording + video editing  | Paid |
| **Jam Deck**   | Apple Music "Now Playing" overlay | Free |

## Option 1: OBS Studio (Free)

Best for: Live recording with real-time "Now Playing" overlay

### Setup

1. **Install OBS Studio**: [obsproject.com](https://obsproject.com/)

2. **Add Browser Source for Slides**:
   - Click `+` in Sources panel
   - Select "Browser"
   - Set URL to your slide deck: `file:///path/to/docs/slides/getting-started.html`
   - Set dimensions to 1920x1080

3. **Add Apple Music "Now Playing" Overlay**:

   **Option A: [Jam Deck](https://github.com/detekoi/jam-deck)** (Recommended for macOS)
   - Install Jam Deck
   - Grant Automation permission: System Preferences → Security & Privacy → Automation
   - Add Browser Source in OBS: `http://localhost:8080/`
   - Position in corner of your scene
   - Themes available: Natural, Twitch, Dark, Pink, Light, Transparent

   **Option B: [Streamling Overlay](https://pengowray.itch.io/streamling)**
   - Supports Apple Music, iTunes, Spotify, YouTube Music, and more
   - Customizable display

   **Option C: [Nutty's Apple Music Widget](https://x.com/nuttylmao/status/1833888351369494815)**
   - Drag-and-drop simple setup

4. **Capture System Audio**:
   - On macOS, you may need [BlackHole](https://existential.audio/blackhole/) or similar virtual audio device
   - Add "Audio Output Capture" source

5. **Scene Layout Example**:

   ```text
   ┌─────────────────────────────────────────┐
   │  reveal.js slideshow (Browser Source)   │
   │                                         │
   │                                         │
   │  ┌─────────────────────────────┐        │
   │  │ 🎵 Song Title - Artist      │        │
   │  │    Now Playing overlay      │        │
   │  └─────────────────────────────┘        │
   └─────────────────────────────────────────┘
   ```

6. **Record**:
   - Settings → Output → Recording Format: mp4
   - Click "Start Recording"
   - Play your Apple Music playlist
   - Let slides auto-advance
   - Click "Stop Recording" when done

---

## Option 2: Camtasia

Best for: Post-production editing, adding music to timeline, precise sync

### Recording the Slideshow

1. **Screen Recording**:
   - Open your reveal.js slide deck in a browser (full screen)
   - Start Camtasia Recorder
   - Select the browser window or full screen
   - Click Record
   - Let slides auto-play through
   - Stop recording

2. **Import to Timeline**:
   - Recording appears on the timeline
   - Separate tracks for video and audio

### Adding Background Music

> **Important**: Apple Music files are DRM-protected (M4P format) and cannot be directly imported into Camtasia. You have two options:

#### Option A: Use Camtasia's Built-in Music Library

- Camtasia includes royalty-free music
- File → Library → Browse built-in assets
- Drag music to timeline below your video

#### Option B: Use Royalty-Free Music

- Sources: [YouTube Audio Library](https://studio.youtube.com/channel/audio), [Pixabay Music](https://pixabay.com/music/), [Free Music Archive](https://freemusicarchive.org/)
- Download MP3/WAV files
- File → Import → Media
- Drag to timeline

#### Option C: Convert Apple Music (if you own the tracks)

- Third-party tools can convert M4P to MP3
- See: [How to Add Apple Music to Camtasia](https://www.noteburner.com/apple-music-tips/add-apple-music-to-camtasia.html)

### Adding a "Now Playing" Overlay in Camtasia

Since Camtasia doesn't have a live "Now Playing" feature, you'll need to add it manually:

1. **Create a Text Callout**:
   - Annotations → Callouts
   - Add a text box with song info
   - Style it to match your slides
   - Position in corner

2. **Or Use an Image Overlay**:
   - Create a PNG with song title/artist in your image editor
   - Import and place on timeline
   - Adjust duration to match song length

### Timeline Sync Tips

- **Default slide duration**: 5 seconds (matches your reveal.js `autoSlide: 5000`)
- **Extend clips**: Drag the end of clips to adjust duration
- **Audio fade**: Apply fade-in/out effects at song transitions
- **Markers**: Use markers to note song change points

### Editing Features

- **Separate tracks**: Screen, camera, system audio, microphone on separate tracks
- **Audio cleanup**: Remove background noise, adjust volume levels
- **Transitions**: Add transitions between sections
- **Callouts**: Add annotations, arrows, highlights
- **Captions**: Auto-transcribe and add closed captions

### Export for YouTube

1. Share → Local File
2. Select MP4 format
3. Resolution: 1080p or 4K
4. Upload to YouTube

---

## Slide Deck Timing Reference

Current configuration: `autoSlide: 5000` (5 seconds per slide)

| Slide Deck                  | Estimated Slides | Duration      | Changes                                         |
| --------------------------- | ---------------- | ------------- | ----------------------------------------------- |
| getting-started.html        | ~45              | ~3 min 45 sec | Added: 6 E2B slides + 5 Backup/Restore slides   |
| extensions.html             | ~39              | ~3 min 15 sec | Added: 1 Docker DinD slide                      |
| workspace-and-projects.html | ~33              | ~2 min 45 sec | No changes                                      |
| **Total**                   | ~117             | **~9 min 45 sec** | **+11 slides, +45 seconds**                 |

To adjust timing, edit the `autoSlide` value in each HTML file:

- `7000` = 7 seconds (~14 min total)
- `10000` = 10 seconds (~19 min 30 sec total)

---

## Copyright Considerations for YouTube

### Apple Music Tracks

- Will likely trigger Content ID claims
- May result in: ads on your video, muted audio, or blocked in some countries
- Original artist gets ad revenue

### Alternatives

- **Camtasia's built-in library**: Royalty-free
- **YouTube Audio Library**: Free, no claims
- **Creative Commons music**: Check license terms
- **Epidemic Sound / Artlist**: Paid subscriptions, YouTube-safe

---

## Quick Start Checklist

### OBS Workflow

- [ ] Install OBS Studio
- [ ] Install Jam Deck (or alternative)
- [ ] Create scene with Browser source (slides)
- [ ] Add Browser source (Now Playing overlay)
- [ ] Configure audio capture
- [ ] Test recording
- [ ] Record final video
- [ ] Upload to YouTube

### Camtasia Workflow

- [ ] Open slides in browser (full screen)
- [ ] Record screen with Camtasia
- [ ] Import recording to timeline
- [ ] Add background music (royalty-free recommended)
- [ ] Add song title overlay manually
- [ ] Edit and polish
- [ ] Export as MP4
- [ ] Upload to YouTube

---

## Resources

- [OBS Studio](https://obsproject.com/)
- [Jam Deck - Apple Music overlay](https://github.com/detekoi/jam-deck)
- [Streamling Overlay](https://pengowray.itch.io/streamling)
- [Music on Stream - OBS overlay](https://obsproject.com/forum/resources/music-on-stream-a-web-based-current-song-now-playing-overlay.1920/)
- [Camtasia Tutorials](https://www.techsmith.com/learn/tutorials/camtasia/)
- [Record PowerPoint in Camtasia](https://www.techsmith.com/learn/tutorials/camtasia/record-a-powerpoint-presentation/)
- [Import Slides to Camtasia](https://www.techsmith.com/learn/tutorials/camtasia/import-powerpoint-slides/)
- [Apple Music to Camtasia](https://www.noteburner.com/apple-music-tips/add-apple-music-to-camtasia.html)
