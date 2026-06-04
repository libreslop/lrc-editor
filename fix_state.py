import os
import re

mappings = {
    r'\.current_time_ms': '.playback.current_time_ms',
    r'\.duration_ms': '.playback.duration_ms',
    r'\.playing': '.playback.playing',
    r'\.last_seek_request': '.playback.last_seek_request',
    r'\.source_text': '.document.source_text',
    r'\.document': '.document.document',
    r'\.parse_error': '.document.parse_error',
    r'\.next_uid': '.document.next_uid',
    r'\.history\b(?!_)': '.history.history',
    r'\.history_index': '.history.history_index',
    r'\.zoom_level': '.view.zoom_level',
    r'\.selection': '.view.selection',
    r'\.audio_filename': '.project.audio_filename',
    r'\.lrc_filename': '.project.lrc_filename',
}

def process_file(filepath):
    with open(filepath, 'r') as f:
        content = f.read()
    
    original = content
    for pattern, repl in mappings.items():
        # Only replace if preceded by state or similar variables?
        # Let's just replace if preceded by `state`, `props.state`, `new_state`, `self`. But wait, in actions.rs we already did it manually.
        # Let's ignore actions.rs.
        if "actions.rs" in filepath:
            continue
            
        # We look for something ending with state or props.state
        # Actually it's easier to just match state\.duration_ms or props.state\.duration_ms etc.
        # we can replace `.foo` with `.category.foo` but we must be careful not to replace things that aren't on state.
        # So we can match `(\bstate|\bprops\.state)\.field`
        pass

    # A safer approach:
    for field, repl in [
        ('current_time_ms', 'playback.current_time_ms'),
        ('duration_ms', 'playback.duration_ms'),
        ('playing', 'playback.playing'),
        ('last_seek_request', 'playback.last_seek_request'),
        ('source_text', 'document.source_text'),
        ('document', 'document.document'),
        ('parse_error', 'document.parse_error'),
        ('next_uid', 'document.next_uid'),
        ('history', 'history.history'),
        ('history_index', 'history.history_index'),
        ('zoom_level', 'view.zoom_level'),
        ('selection', 'view.selection'),
        ('audio_filename', 'project.audio_filename'),
        ('lrc_filename', 'project.lrc_filename'),
    ]:
        if "actions.rs" in filepath: continue
        content = re.sub(r'(\bstate|\bprops\.state|\bnew_state)\.' + field + r'\b', r'\1.' + repl, content)

    if content != original:
        with open(filepath, 'w') as f:
            f.write(content)

for root, _, files in os.walk('src'):
    for file in files:
        if file.endswith('.rs'):
            process_file(os.path.join(root, file))

