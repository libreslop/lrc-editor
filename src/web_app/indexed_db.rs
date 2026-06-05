use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

#[wasm_bindgen(inline_js = r#"
const DB_NAME = "lrc-editor-audio-db";
const STORE_NAME = "audio-store";
const DB_VERSION = 1;

function getDB() {
    return new Promise((resolve, reject) => {
        const request = indexedDB.open(DB_NAME, DB_VERSION);
        request.onerror = () => reject(request.error);
        request.onsuccess = () => resolve(request.result);
        request.onupgradeneeded = (event) => {
            const db = request.result;
            if (!db.objectStoreNames.contains(STORE_NAME)) {
                db.createObjectStore(STORE_NAME);
            }
        };
    });
}

export async function store_audio_file(name, file) {
    const db = await getDB();
    return new Promise((resolve, reject) => {
        const tx = db.transaction(STORE_NAME, "readwrite");
        const store = tx.objectStore(STORE_NAME);
        const request = store.put({ name, file }, "current");
        request.onerror = () => reject(request.error);
        request.onsuccess = () => resolve();
    });
}

export async function load_audio_file_js() {
    const db = await getDB();
    return new Promise((resolve, reject) => {
        const tx = db.transaction(STORE_NAME, "readonly");
        const store = tx.objectStore(STORE_NAME);
        const request = store.get("current");
        request.onerror = () => reject(request.error);
        request.onsuccess = () => {
            const result = request.result;
            if (result) {
                resolve(result);
            } else {
                resolve(null);
            }
        };
    });
}

export async function clear_audio_file() {
    const db = await getDB();
    return new Promise((resolve, reject) => {
        const tx = db.transaction(STORE_NAME, "readwrite");
        const store = tx.objectStore(STORE_NAME);
        const request = store.delete("current");
        request.onerror = () => reject(request.error);
        request.onsuccess = () => resolve();
    });
}
"#)]
extern "C" {
    #[wasm_bindgen(catch)]
    pub async fn store_audio_file(name: String, file: &web_sys::Blob) -> Result<(), JsValue>;

    #[wasm_bindgen(catch)]
    async fn load_audio_file_js() -> Result<JsValue, JsValue>;

    #[wasm_bindgen(catch)]
    pub async fn clear_audio_file() -> Result<(), JsValue>;
}

pub struct SavedAudio {
    pub name: String,
    pub blob: web_sys::Blob,
}

pub async fn load_audio_file() -> Option<SavedAudio> {
    if let Ok(js_val) = load_audio_file_js().await {
        if !js_val.is_null() && !js_val.is_undefined() {
            let name = js_sys::Reflect::get(&js_val, &JsValue::from_str("name"))
                .ok()?
                .as_string()?;
            let file = js_sys::Reflect::get(&js_val, &JsValue::from_str("file"))
                .ok()?
                .dyn_into::<web_sys::Blob>()
                .ok()?;
            return Some(SavedAudio { name, blob: file });
        }
    }
    None
}
