use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct TrackPadsProps {
    pub on_import_audio: Callback<MouseEvent>,
    pub on_import_lrc: Callback<MouseEvent>,
    pub on_export_lrc: Callback<MouseEvent>,
}

#[function_component(TrackPads)]
pub fn track_pads(props: &TrackPadsProps) -> Html {
    html! {
        <div class="track-pads">
            <div class="track-pad ruler-pad"></div>
            <div class="track-pad audio-pad centered">
                <button class="icon-button track-button" title="Import Audio" onclick={props.on_import_audio.clone()}>
                    <svg viewBox="0 0 24 24"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="17 8 12 3 7 8"/><line x1="12" y1="3" x2="12" y2="15"/></svg>
                </button>
            </div>
            <div class="track-pad lyrics-pad side-by-side">
                <button class="icon-button track-button" title="Import LRC" onclick={props.on_import_lrc.clone()}>
                    <svg viewBox="0 0 24 24"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="17 8 12 3 7 8"/><line x1="12" y1="3" x2="12" y2="15"/></svg>
                </button>
                <button class="icon-button track-button" title="Export LRC" onclick={props.on_export_lrc.clone()}>
                    <svg viewBox="0 0 24 24"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg>
                </button>
            </div>
        </div>
    }
}
