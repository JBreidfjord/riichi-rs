use once_cell::sync::Lazy;
use url::Url;

macro_rules! url {
    ($x:expr) => { Lazy::<Url>::new(|| Url::parse($x).unwrap()) }
}

pub static TENHOU_ARCHIVE_ROOT: Lazy<Url> = url!("https://tenhou.net/sc/raw/dat/");
pub static TENHOU_LIST_RECENT: Lazy<Url> = url!("https://tenhou.net/sc/raw/list.cgi");
pub static TENHOU_LIST_YEAR_TO_DATE: Lazy<Url> = url!("https://tenhou.net/sc/raw/list.cgi?old");
pub static TENHOU_REFERER: Lazy<Url> = url!("https://tenhou.net/6/?");
pub static TENHOU_DOWNLOAD: Lazy<Url> = url!("https://tenhou.net/5/mjlog2json.cgi");

pub fn tenhou_download_url(id: &str) -> Url {
    let mut url = TENHOU_DOWNLOAD.clone();
    url.query_pairs_mut().append_key_only(id);
    url
}
