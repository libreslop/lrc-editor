build a lrc file editor, an lrc file is a synced lyrics file, i have included an example of an lcs file in the currrent directory

technology - single page web application with webassembly

style of the editor inspired by a video editor

* the screen is split into 2 parts - horizontal split top and bottom

   * on the top split - vertically split it into 2

      * top left: the text field containing the current file content, above the text field there are icon buttons for undo/redo, copy, import, export (using name of imported file, otherwise using name of the imported audio) for the lcs file

      * top right: displays the synced lyrics focusing on the current line of lyrics, allow the following - scrolling on the synced lyrics field will detach scrolling from being focused on the current line, it shows a "resume autoscroll" button. clicking on a line of lyrics jumps to that timestamp

   * on the bottom split, it is a timeline with two tracks, each track has a padding to the left showing an icon

      * the first track is the audio track, when there is no audio file selected, the whole track should be a giant button saying "import audio", the audio track should show the waveform of the audio (load it progressively from the start of the audio, so to not block the rendering thread or cause freezing), the audio is fixed in position and cannot be shifted

      * the audio track left padding shows an upload button (in icon) at all times

      * the second track is the lyrics track, each line of lyric is represented on the timeline as a chunk (a rectangular thing), on the chunk it should have the words of that line in a single line, if its too long, cut it off with "..." at the end

      * the left pad of the lyrics track is a select all button (in icon)

      * there should be a play head (a red line with a marker on top)



other behaviours:



* when clicking on the synced lyrics preview, or selecting a chunk, or the playhead moving causing it to now hover on a different chunk, basically when any update in player position happens

   * the cursor in synced lyrics, player head, and selected chunk should all jump to the line of the current lyrics, so everything is always in sync

   * select the lyrics (line content) portion of the lcs on the textfield, but do not focus on the text field if it is not already is (because the user may be using arrow keys to adjust player head)

* allow using arrow keys at adjust player head

* do not show chunks for empty lines, in the track for lyrics, allow empty spaces between chunks

* focus on the textfield on hover (so key inputs goes into there), unfocus from the textfield on unhover, when textfield is not in focus, key inputs goes to the track/playerhead

* allow selected, selecting multiple - with shift to expand selection to include regions between the clicked chunk and any previous selected chunk, with control to toggle selection of any particular chunk between selected and not selected - when multiple chunks are selected, do not trigger an update on textfield selection

* below the tracks, show a horizontal scroll bar for scrolling the tracks, on the same line as the scrollbar, to the right of it, show a magnifying glass icon +/- for zoom in/out on the timeline

* any updates in the text field should be reflected in timeline and synced lyrics immediately, if the textfeild is invalid, show a red border and a toast below the textfield instead, until it is resolved

i have included an image of audacity 4, that is the style of the html app you are aiming for
