use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct TrackPadsProps {
    pub on_import_audio: Callback<MouseEvent>,
    pub on_select_all: Callback<MouseEvent>,
}

#[function_component(TrackPads)]
pub fn track_pads(props: &TrackPadsProps) -> Html {
    html! {
        <div class="track-pads">
            <div class="track-pad ruler-pad"></div>
            <div class="track-pad audio-pad">
                <button class="icon-button track-button" title="Import Audio" onclick={props.on_import_audio.clone()}>
                    <svg viewBox="0 0 24 24"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="17 8 12 3 7 8"/><line x1="12" y1="3" x2="12" y2="15"/></svg>
                </button>
            </div>
            <div class="track-pad lyrics-pad">
                <button class="icon-button track-button" title="Select All" onclick={props.on_select_all.clone()}>
                    <svg viewBox="0 0 24 24"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><line x1="16" y1="13" x2="8" y2="13"/><line x1="16" y1="17" x2="8" y2="17"/><polyline points="10 9 9 9 8 9"/></svg>
                </button>
            </div>
        </div>
    }
}
