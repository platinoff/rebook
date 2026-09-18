//! EPUB viewer with KDP EPUB 3.2 compliance preview.
//!
//! Reads a built EPUB (`build/rust_book.epub`) in memory and serves it over a
//! local `127.0.0.1` tokio TCP socket so the book can be previewed the way
//! Amazon KDP would see it. Also exposes a KDP-acceptance check (`/check`).
//!
//! Rust-only, no new dependencies, works fully offline.
use std::io::Read;

use crate::{Book, ChapterMeta};

/// Default bind address for the local preview server.
pub const DEFAULT_ADDR: &str = "127.0.0.1:8090";

/// An EPUB unpacked into memory, keyed by archive entry path.
#[derive(Debug, Clone)]
pub struct Epub {
    entries: Vec<(String, Vec<u8>)>,
}

impl Epub {
    /// Read every entry of an EPUB (zip) into memory.
    pub fn from_path(path: &str) -> Result<Epub, String> {
        let file = std::fs::File::open(path)
            .map_err(|e| format!("EPUB file not found: {} ({})", path, e))?;
        let mut archive =
            zip::ZipArchive::new(file).map_err(|e| format!("Not a valid ZIP: {}", e))?;
        let mut entries = Vec::with_capacity(archive.len());
        for i in 0..archive.len() {
            let mut f = archive
                .by_index(i)
                .map_err(|e| format!("Cannot read entry {i}: {e}"))?;
            let mut bytes = Vec::new();
            f.read_to_end(&mut bytes)
                .map_err(|e| format!("Cannot read {}: {}", f.name(), e))?;
            entries.push((f.name().to_string(), bytes));
        }
        Ok(Epub { entries })
    }

    /// Build an in-memory EPUB from raw entries (tests, generated books).
    #[doc(hidden)]
    pub fn from_entries(entries: Vec<(String, Vec<u8>)>) -> Epub {
        Epub { entries }
    }

    /// Number of stored entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// True iff the archive has no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Look up the bytes of one entry (exact path match).
    pub fn get(&self, name: &str) -> Option<Vec<u8>> {
        self.entries
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, b)| b.clone())
    }

    /// UTF-8 text of one entry, if present and valid.
    pub fn text(&self, name: &str) -> Option<String> {
        self.get(name).and_then(|b| String::from_utf8(b).ok())
    }

    /// Iterate all archive entries (path, bytes) — for cover extraction.
    pub fn entries_iter(&self) -> impl Iterator<Item = &(String, Vec<u8>)> {
        self.entries.iter()
    }

    /// Names of all chapter pages matching `OEBPS/chapter-*.xhtml`.
    pub fn chapter_names(&self) -> Vec<String> {
        let mut v: Vec<String> = self
            .entries
            .iter()
            .filter(|(n, _)| n.starts_with("OEBPS/chapter-") && n.ends_with(".xhtml"))
            .map(|(n, _)| n.clone())
            .collect();
        v.sort();
        v
    }

    /// Auto-derive a `Book` straight from the EPUB package (no `book.json`).
    ///
    /// Reads title / creator / language from `content.opf`, and walks the
    /// spine (`<itemref idref="…">` → manifest `href`) to build the ordered
    /// chapter list, taking each chapter's `<title>` from its own XHTML.
    /// This is what lets any third-party EPUB be previewed without us writing
    /// a `book.json` for it.
    pub fn parse_opf_book(&self) -> Result<Book, String> {
        let opf = self
            .text("OEBPS/content.opf")
            .ok_or("EPUB містить OEBPS/content.opf у недоступному форматі")?;
        let title = text_between(&opf, "<dc:title>", "</dc:title>")
            .unwrap_or("(без назви)")
            .trim()
            .to_string();
        let author = text_between(&opf, "<dc:creator>", "</dc:creator>")
            .unwrap_or("(невідомий автор)")
            .trim()
            .to_string();
        let language = text_between(&opf, "<dc:language>", "</dc:language>")
            .unwrap_or("uk")
            .trim()
            .to_string();
        let isbn = opf.find("urn:isbn:").and_then(|i| {
            let rest = &opf[i + 9..];
            let end = rest.find(['<', '"', ' ']).unwrap_or(rest.len());
            let raw: String = rest[..end]
                .chars()
                .filter(|c| c.is_ascii_digit() || *c == 'X')
                .collect();
            (!raw.is_empty()).then_some(raw)
        });

        // idref list from the spine, in order.
        let mut idrefs = Vec::new();
        let mut rest = opf.as_str();
        while let Some(i) = rest.find("<itemref") {
            let seg = &rest[i..];
            let end = seg.find('>').unwrap_or(seg.len());
            let tag = &seg[..end];
            if let Some(id) = attr_value(tag, "idref") {
                idrefs.push(id.to_string());
            }
            rest = &seg[end + 1..];
        }

        // Manifest: id → href/file.
        let mut href_by_id: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        rest = opf.as_str();
        while let Some(i) = rest.find("<item ") {
            let seg = &rest[i..];
            let end = seg.find('>').unwrap_or(seg.len());
            let tag = &seg[..end];
            if let (Some(id), Some(href)) = (attr_value(tag, "id"), attr_value(tag, "href")) {
                href_by_id.insert(id.to_string(), href.to_string());
            }
            rest = &seg[end + 1..];
        }

        // Resolve href → entry path (OEBPS/ prefix).
        let mut chapters = Vec::new();
        let mut num = 1u32;
        for id in &idrefs {
            let Some(href) = href_by_id.get(id) else {
                continue;
            };
            if href.ends_with(".xhtml") {
                let entry = if href.starts_with("OEBPS/") {
                    href.clone()
                } else {
                    format!("OEBPS/{href}")
                };
                if let Some(doc) = self.text(&entry) {
                    let ctitle = text_between(&doc, "<title>", "</title>")
                        .or_else(|| text_between(&doc, "<h1>", "</h1>"))
                        .unwrap_or(&format!("Розділ {num}"))
                        .trim()
                        .to_string();
                    chapters.push(ChapterMeta {
                        number: num,
                        title: ctitle,
                        file: entry,
                    });
                    num += 1;
                }
            }
        }

        if chapters.is_empty() {
            return Err("Не знайдено жодного XHTML у spine".to_string());
        }

        Ok(Book {
            title,
            author,
            edition: 1,
            year: 0,
            format: "EPUB 3.2".to_string(),
            language,
            isbn,
            chapters,
        })
    }
}

/// Value of an attribute (`name="value"`) inside an HTML/XML tag string.
fn attr_value(tag: &str, name: &str) -> Option<String> {
    let key = format!("{name}=\"");
    let start = tag.find(&key)? + key.len();
    let rel = &tag[start..];
    let end = rel.find('"')?;
    Some(unescape_xml(rel[..end].trim()))
}

/// The text between two literal markers (first occurrence), if any.
fn text_between<'a>(s: &'a str, open: &str, close: &str) -> Option<&'a str> {
    let start = s.find(open)? + open.len();
    let end = s[start..].find(close)? + start;
    Some(&s[start..end])
}

/// Decode the handful of XML entities we emit / commonly see in OPF titles.
fn unescape_xml(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}

/// Individual result line of the KDP compliance check.
#[derive(Debug, Clone, serde::Serialize)]
pub struct KdpCheckItem {
    pub name: String,
    pub ok: bool,
    pub detail: String,
}

/// Result of a KDP-compliance scan over an EPUB.
#[derive(Debug, Clone)]
pub struct KdpReport {
    pub items: Vec<KdpCheckItem>,
}

impl KdpReport {
    /// True iff every item passed.
    pub fn passed(&self) -> bool {
        self.items.iter().all(|i| i.ok)
    }

    /// Count of passed items.
    pub fn passes(&self) -> usize {
        self.items.iter().filter(|i| i.ok).count()
    }
}

/// Run the KDP EPUB 3.2 compliance check over an opened EPUB.
pub fn kdp_check(epub: &Epub, book: &Book) -> KdpReport {
    let mut items = Vec::new();

    // 1. mimetype must be the first entry, stored, exact content.
    let first = epub.entries.first();
    let mimetype_ok = match first {
        Some((name, bytes)) => name == "mimetype" && bytes == b"application/epub+xml",
        None => false,
    };
    items.push(mimetype_item(mimetype_ok));

    // 2. Required entries.
    for want in [
        "mimetype",
        "META-INF/container.xml",
        "OEBPS/content.opf",
        "OEBPS/nav.xhtml",
        "OEBPS/styles.css",
    ] {
        let ok = epub.get(want).is_some();
        items.push(KdpCheckItem {
            name: format!("entry:{want}"),
            ok,
            detail: if ok {
                "присутній".to_string()
            } else {
                "ПРОПУЩЕНО".to_string()
            },
        });
    }

    // 3. OPF metadata sanity + spine/ref-count vs chapters.
    let chapters = epub.chapter_names();
    let opf = epub.text("OEBPS/content.opf").unwrap_or_default();
    for (key, needle, label) in [
        ("opf:dc:title", "<dc:title>", "назва"),
        ("opf:dc:creator", "<dc:creator>", "автор"),
        ("opf:dc:language", "<dc:language>", "мова"),
        ("opf:dc:identifier", "urn:uuid:", "uid"),
        ("opf:dc:date", "<dc:date>", "дата"),
        ("opf:modified", "dcterms:modified", "час редагування"),
        ("opf:spine", "<spine", "поява <spine>"),
    ] {
        let ok = opf.contains(needle);
        items.push(KdpCheckItem {
            name: key.to_string(),
            ok,
            detail: if ok {
                label.to_string()
            } else {
                format!("ПРОПУЩЕНО (без {needle})")
            },
        });
    }

    // Book chapters from JSON vs entries in the EPUB.
    let expected = book.chapters.len();
    let found = chapters.len();
    items.push(KdpCheckItem {
        name: "chapters:count".to_string(),
        ok: expected == found,
        detail: format!("book.json={expected}, epub={found}"),
    });

    // Each expected chapter page present.
    for meta in &book.chapters {
        let want = format!("OEBPS/chapter-{:02}.xhtml", meta.number);
        let ok = epub.get(&want).is_some();
        items.push(KdpCheckItem {
            name: format!("chapter:{:02}", meta.number),
            ok,
            detail: if ok {
                meta.title.clone()
            } else {
                "ПРОПУЩЕНО".to_string()
            },
        });
    }

    // 4. Nav has toc + chapter links.
    let nav = epub.text("OEBPS/nav.xhtml").unwrap_or_default();
    items.push(KdpCheckItem {
        name: "nav:toc".to_string(),
        ok: nav.contains("epub:type=\"toc\""),
        detail: "epub:type=\"toc\"".to_string(),
    });
    items.push(KdpCheckItem {
        name: "nav:links".to_string(),
        ok: nav.contains("chapter-"),
        detail: "посилання chapter-".to_string(),
    });

    // 5. KDP-forbidden constructs across all XHTML + CSS.
    let mut forbidden_html: Vec<String> = Vec::new();
    for (name, _) in &epub.entries {
        if !name.ends_with(".xhtml") {
            continue;
        }
        let body = String::from_utf8_lossy(&epub.get(name).unwrap_or_default()).to_string();
        let low = body.to_ascii_lowercase();
        for tag in [
            "<script", "<iframe", "<video", "<audio", "<form", "onclick", "onload",
        ] {
            if low.contains(tag) {
                forbidden_html.push(format!("{name}: {tag}"));
            }
        }
        // raw unescaped ampersands (crude well-formedness signal)
        if raw_ampersand(&body) {
            forbidden_html.push(format!("{name}: сирий &"));
        }
        // External absolute URLs in resource-loading attributes (href/src) —
        // these pull web resources at render time and KDP rejects them.
        // XML namespace declarations (xmlns="http://…") are structural and fine.
        for found in external_resource_urls(&body) {
            forbidden_html.push(format!("{name}: зовнішня {found}"));
        }
    }
    items.push(KdpCheckItem {
        name: "xhtml:forbidden".to_string(),
        ok: forbidden_html.is_empty(),
        detail: if forbidden_html.is_empty() {
            "немає заборонених конструкцій".to_string()
        } else {
            forbidden_html.join("; ")
        },
    });

    let css = epub.text("OEBPS/styles.css").unwrap_or_default();
    let css_low = css.to_ascii_lowercase();
    let mut css_problems: Vec<String> = Vec::new();
    if css_low.contains("@import") {
        css_problems.push("@import".to_string());
    }
    if css_low.contains("url(") {
        css_problems.push("url() у стилях".to_string());
    }
    items.push(KdpCheckItem {
        name: "css:no-external".to_string(),
        ok: css_problems.is_empty(),
        detail: if css_problems.is_empty() {
            "стилі без зовнішніх".to_string()
        } else {
            css_problems.join("; ")
        },
    });

    // 6. ISBN in OPF: required when book.json declares one, or for EN editions.
    if let Some(item) = opf_isbn_item(&opf, book) {
        items.push(item);
    }

    KdpReport { items }
}

/// `opf:isbn` — EN editions and any `book.json` ISBN must land as `urn:isbn:`.
fn opf_isbn_item(opf: &str, book: &Book) -> Option<KdpCheckItem> {
    let lang = book.language.to_ascii_lowercase();
    let en = lang == "en" || lang.starts_with("en-");
    let declared = book
        .isbn
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    if declared.is_none() && !en {
        return None;
    }
    if let Some(raw) = declared {
        let ok = match crate::standards::isbn_to_ean13(raw) {
            Ok(ean) => opf.contains(&format!("urn:isbn:{ean}")),
            Err(_) => false,
        };
        return Some(KdpCheckItem {
            name: "opf:isbn".to_string(),
            ok,
            detail: if ok {
                format!("urn:isbn:{raw}")
            } else {
                format!("book.json ISBN {raw} немає в OPF як urn:isbn:")
            },
        });
    }
    let has = opf.contains("urn:isbn:");
    Some(KdpCheckItem {
        name: "opf:isbn".to_string(),
        ok: has,
        detail: if has {
            "urn:isbn у OPF".to_string()
        } else {
            "EN edition: задайте book.json isbn, щоб OPF мав urn:isbn:".to_string()
        },
    })
}

fn mimetype_item(ok: bool) -> KdpCheckItem {
    KdpCheckItem {
        name: "mimetype:first-stored".to_string(),
        ok,
        detail: if ok {
            "перший запис, без стиск, application/epub+xml".to_string()
        } else {
            "mimetype не перший/не Stored/непривильний вміст".to_string()
        },
    }
}

/// True if the string has a `&` not followed by a known XML entity name.
fn raw_ampersand(s: &str) -> bool {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'&' {
            let rest = &s[i + 1..];
            let prefix = rest
                .split(|c: char| !c.is_ascii_alphanumeric())
                .next()
                .unwrap_or("");
            if !matches!(prefix, "amp" | "lt" | "gt" | "quot" | "apos") {
                return true;
            }
        }
        i += 1;
    }
    false
}

/// Find absolute `http(s)://` URLs inside `href=""` / `src=""` attribute values.
///
/// Namespace declarations (`xmlns="http://…"`) are structural XML and are not
/// external resource loads, so they are ignored here.
fn external_resource_urls(html: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bytes = html.as_bytes();
    let n = bytes.len();
    let mut i = 0;
    while i < n {
        // Only ASCII word characters/punct form an attribute token.
        let start = i;
        let mut j = i;
        while j < n && is_attr_char(bytes[j]) {
            j += 1;
        }
        let len = j - start;
        let is_href_or_src =
            len == 4 && (&bytes[start..j] == b"href") || len == 3 && (&bytes[start..j] == b"src");
        i = j;
        // Skip whitespace, expect '=' then the quoted/unquoted value.
        while i < n && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i < n && bytes[i] == b'=' {
            i += 1;
            while i < n && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            let quote = if i < n && (bytes[i] == b'"' || bytes[i] == b'\'') {
                let q = bytes[i];
                i += 1;
                Some(q)
            } else {
                None
            };
            let val_start = i;
            let val_end;
            if let Some(q) = quote {
                while i < n && bytes[i] != q {
                    i += 1;
                }
                val_end = i;
                // step past the closing quote so the outer loop advances
                if i < n {
                    i += 1;
                }
            } else {
                while i < n && !bytes[i].is_ascii_whitespace() {
                    i += 1;
                }
                val_end = i;
            }
            let val = &bytes[val_start..val_end];
            if is_href_or_src && (val.starts_with(b"http://") || val.starts_with(b"https://")) {
                out.push(String::from_utf8_lossy(val).into_owned());
            }
        } else if len == 0 {
            // no '=' and no real token: advance one byte so we never stall
            i += 1;
        }
    }
    out
}

/// True for bytes that can appear in an HTML attribute name.
fn is_attr_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'-'
}

/// Render the page for a book index (`/`  or `/{id}/`).
/// One top-nav zone: href + uk/en labels + GSV-style hover tips.
pub struct NavItem {
    /// Path this tab opens.
    pub href: &'static str,
    /// Ukrainian label.
    pub uk: &'static str,
    /// English label.
    pub en: &'static str,
    /// Hover tip (uk).
    pub tip_uk: &'static str,
    /// Hover tip (en).
    pub tip_en: &'static str,
}

/// The six primary zones for the shared top navigation.
pub const NAV_ITEMS: [NavItem; 6] = [
    NavItem {
        href: "/",
        uk: "Полиця",
        en: "Shelf",
        tip_uk: "Читалка EPUB і звіт KDP-перевірок",
        tip_en: "EPUB reader and KDP checklist",
    },
    NavItem {
        href: "/studio",
        uk: "Studio",
        en: "Studio",
        tip_uk: "Чернетки: пиши своєю мовою, потім переклади",
        tip_en: "Drafts: write in your language, then translate",
    },
    NavItem {
        href: "/cover",
        uk: "Обкладинки",
        en: "Covers",
        tip_uk: "Шаблони wrap: ebook / paperback / hardcover",
        tip_en: "Wrap templates: ebook / paperback / hardcover",
    },
    NavItem {
        href: "/kdp",
        uk: "KDP",
        en: "KDP",
        tip_uk: "Локальний Print Previewer: wrap + interior, гайди, розмір файлів",
        tip_en: "Local Print Previewer: wrap + interior, guides, file size",
    },
    NavItem {
        href: "/view3d",
        uk: "3D",
        en: "3D",
        tip_uk: "Віртуальний стенд книги (ebook + м'яка + тверда)",
        tip_en: "Virtual book stand (ebook + paperback + hardcover)",
    },
    NavItem {
        href: "/products",
        uk: "Продукти",
        en: "Products",
        tip_uk: "Готові пакети для завантаження на KDP",
        tip_en: "Finished packages for KDP upload",
    },
];

const NAV_STYLE: &str = "\
.rb-nav{position:sticky;top:0;z-index:7;display:flex;gap:.9em;align-items:center;padding:.5em .9em;background:#161922;border-bottom:1px solid #2f3542;font:600 14px/1.4 system-ui,'Segoe UI',sans-serif;color:#9aa0ac}\
.rb-nav .rb-brand{color:#7aa2f7;font-weight:800;letter-spacing:.04em;cursor:default}\
.rb-nav a{color:#9aa0ac;text-decoration:none;padding:.2em .1em;border-bottom:2px solid transparent}\
.rb-nav a:hover{color:#e6e6e6}\
.rb-nav a.active{color:#7aa2f7;border-bottom-color:#7aa2f7}\
.rb-nav .rb-home{margin-left:auto;font-weight:400;font-size:.85em}\
.rb-nav .rb-lang{cursor:pointer;user-select:none;border:1px solid #2f3542;border-radius:999px;padding:.12em .7em;font-size:.85em}\
.rb-nav .rb-lang:hover{border-color:#7aa2f7;color:#e6e6e6}\
#rbTip{position:fixed;z-index:2000;max-width:280px;padding:6px 8px;border-radius:6px;border:1px solid #2f3542;background:#1a2233;color:#e6e6e6;font:12px/1.35 system-ui,'Segoe UI',sans-serif;display:none;pointer-events:none;box-shadow:0 8px 24px rgba(0,0,0,.45)}\
.rb-box{position:relative}\
.rb-box>.rb-box-fs{position:absolute;top:8px;right:8px;z-index:6;width:28px;height:28px;border:1px solid #2f3542;border-radius:6px;background:#161922cc;color:#7aa2f7;cursor:pointer;font:700 14px/1 system-ui}\
.rb-box>.rb-box-fs:hover{border-color:#7aa2f7;color:#e6e6e6}\
.rb-box.fullscreen{position:fixed!important;inset:10px;top:48px;z-index:80!important;width:auto!important;height:auto!important;max-width:none!important;max-height:none!important;min-height:0!important;margin:0!important;border-radius:10px;box-shadow:0 24px 80px rgba(0,0,0,.55)}\
body.rb-box-fs{overflow:hidden}\
body.rb-box-fs .rb-nav{z-index:90}";

const NAV_JS: &str = r#"(function(){
var lang=localStorage.getItem('rb.lang')||'uk';
function apply(){
 document.documentElement.lang=lang;
 document.querySelectorAll('[data-uk][data-en]').forEach(function(el){
  if(el.querySelector&&el.querySelector('input,select,textarea'))return;
  var v=lang==='en'?el.getAttribute('data-en'):el.getAttribute('data-uk');
  if(v!==null)el.textContent=v;
  var t=lang==='en'?el.getAttribute('data-tip-en'):el.getAttribute('data-tip-uk');
  if(t){el.setAttribute('data-tip',t);el.setAttribute('title',t);}
 });
 var chip=document.getElementById('rbLangChip');
 if(chip)chip.textContent=lang==='uk'?'EN':'УК';
 document.querySelectorAll('[data-action=box-fs]').forEach(function(el){
  var t=lang==='en'?el.getAttribute('data-tip-en'):el.getAttribute('data-tip-uk');
  if(t)el.setAttribute('data-tip',t);
 });
}
window.rbSetLang=function(l){lang=l;localStorage.setItem('rb.lang',l);apply();document.dispatchEvent(new CustomEvent('rb-lang',{detail:l}));};
apply();
document.addEventListener('DOMContentLoaded',apply);
var chip=document.getElementById('rbLangChip');
if(chip)chip.onclick=function(){window.rbSetLang(lang==='uk'?'en':'uk');};
var tip=document.getElementById('rbTip');
if(tip){
 var place=function(e){tip.style.left=Math.min(e.clientX+12,innerWidth-292)+'px';tip.style.top=Math.min(e.clientY+16,innerHeight-72)+'px';};
 document.addEventListener('pointerover',function(e){
  var el=e.target.closest&&e.target.closest('[data-tip],[title]');
  if(!el){tip.style.display='none';return;}
  var t=el.getAttribute('data-tip');
  if(!t&&el.hasAttribute('title')){t=el.getAttribute('title');el.setAttribute('data-tip',t);el.removeAttribute('title');}
  if(!t){tip.style.display='none';return;}
  tip.textContent=t;tip.style.display='block';place(e);
 });
 document.addEventListener('pointermove',function(e){if(tip.style.display==='block')place(e);});
 document.addEventListener('pointerout',function(e){if(!e.relatedTarget||!e.relatedTarget.closest('[data-tip]'))tip.style.display='none';});
}
function rbExitBoxFs(){
 document.querySelectorAll('.rb-box.fullscreen').forEach(function(b){
  b.classList.remove('fullscreen');
  var btn=b.querySelector('[data-action=box-fs]');
  if(btn){
   btn.textContent='□';
   var tu=btn.getAttribute('data-tip-uk')||'';
   var te=btn.getAttribute('data-tip-en')||'';
   btn.setAttribute('data-tip',lang==='en'?te:tu);
  }
 });
 document.body.classList.remove('rb-box-fs');
 window.dispatchEvent(new Event('resize'));
}
window.rbExitBoxFs=rbExitBoxFs;
window.rbBindBoxFs=function(){
 document.querySelectorAll('.rb-box').forEach(function(box){
  if(box.querySelector('[data-action=box-fs]'))return;
  var btn=document.createElement('button');
  btn.type='button';
  btn.className='rb-box-fs';
  btn.setAttribute('data-action','box-fs');
  btn.textContent='□';
  btn.setAttribute('data-tip-uk','На весь екран (Esc)');
  btn.setAttribute('data-tip-en','Fullscreen (Esc)');
  btn.setAttribute('data-tip',lang==='en'?'Fullscreen (Esc)':'На весь екран (Esc)');
  btn.setAttribute('aria-label','Fullscreen');
  btn.onclick=function(ev){
   ev.stopPropagation();
   var on=!box.classList.contains('fullscreen');
   rbExitBoxFs();
   if(on){
    box.classList.add('fullscreen');
    btn.textContent='×';
    document.body.classList.add('rb-box-fs');
    window.dispatchEvent(new Event('resize'));
   }
  };
  box.appendChild(btn);
 });
};
window.rbBindBoxFs();
document.addEventListener('DOMContentLoaded',window.rbBindBoxFs);
if(document.body){
 new MutationObserver(function(){window.rbBindBoxFs();}).observe(document.body,{childList:true,subtree:true});
}
document.addEventListener('keydown',function(e){
 if(e.key!=='Escape')return;
 if(document.body.classList.contains('rb-box-fs')){
  rbExitBoxFs();
  e.preventDefault();
  e.stopPropagation();
 }
},true);
})();"#;

/// A compact, self-contained top nav bar. `active` is the current path so
/// its link is highlighted. Uses only class hooks styled by `viewer_css`.
pub fn nav_html(active: &str) -> String {
    let links: Vec<String> = NAV_ITEMS
        .iter()
        .map(|it| {
            let cls = if it.href == active {
                " class=\"active\""
            } else {
                ""
            };
            format!(
                "<a{cls} href=\"{href}\" data-uk=\"{uk}\" data-en=\"{en}\" data-tip=\"{tu}\" data-tip-uk=\"{tu}\" data-tip-en=\"{te}\">{uk}</a>",
                href = it.href,
                uk = it.uk,
                en = it.en,
                tu = it.tip_uk,
                te = it.tip_en,
            )
        })
        .collect();
    format!(
        "<style>{style}</style>\
         <nav class=\"rb-nav\" aria-label=\"rebook\">\
         <span class=\"rb-brand\" data-uk=\"rebook\" data-en=\"rebook\" data-tip=\"rebook — EPUB 3.2 + KDP studio\" data-tip-uk=\"rebook — EPUB 3.2 + KDP studio\" data-tip-en=\"rebook — EPUB 3.2 + KDP studio\">rebook</span>\
         {links}\
         <span class=\"rb-lang\" id=\"rbLangChip\" data-tip=\"Мова інтерфейсу: українська / English\" data-tip-uk=\"Мова інтерфейсу: українська / English\" data-tip-en=\"Interface language: Ukrainian / English\">EN</span>\
         <a class=\"rb-home\" href=\"/books\" data-uk=\"усі книги →\" data-en=\"all books →\" data-tip=\"Повна полиця EPUB\" data-tip-uk=\"Повна полиця EPUB\" data-tip-en=\"Full EPUB shelf\">усі книги →</a>\
         </nav>\
         <div id=\"rbTip\" role=\"tooltip\"></div>\
         <script>{js}</script>",
        style = NAV_STYLE,
        links = links.join(""),
        js = NAV_JS,
    )
}

/// Inject the shared nav into a static page at its `<!--rbnav-->` marker.
pub fn inject_nav(html: &str, active: &str) -> String {
    html.replace("<!--rbnav-->", &nav_html(active))
}

fn render_index(epub: &Epub, book: &Book, book_id: &str) -> String {
    let report = kdp_check(epub, book);
    let passed = report.passed();
    let chapters = epub.chapter_names();

    let mut rows = String::new();
    for item in &report.items {
        let mark = if item.ok { "&#10003;" } else { "&#10007;" };
        let cls = if item.ok { "ok" } else { "fail" };
        rows.push_str(&format!(
            "    <tr class=\"{cls}\"><td>{mark}</td><td>{}</td><td>{}</td></tr>\n",
            html_esc(&item.name),
            html_esc(&item.detail)
        ));
    }

    let mut toc = String::new();
    for meta in &book.chapters {
        let present = epub
            .get(&format!("OEBPS/chapter-{:02}.xhtml", meta.number))
            .is_some();
        let link = if present {
            format!(
                "<a href=\"/{id}/chapter/{}\">Розділ {} — {}</a>",
                meta.number,
                meta.number,
                html_esc(&meta.title),
                id = book_id,
            )
        } else {
            format!(
                "Розділ {} — {} <em>(немає в epub)</em>",
                meta.number,
                html_esc(&meta.title)
            )
        };
        toc.push_str(&format!("    <li>{}</li>\n", link));
    }

    let badge_out = if passed { "PASS" } else { "FAIL" };
    let badge_cls = if passed { "pass" } else { "fail" };

    format!(
        r#"<!DOCTYPE html>
<html lang="uk">
<head>
<meta charset="utf-8"/>
<title>{title} — EPUB просмоторщик</title>
<link rel="stylesheet" href="/styles.css"/>
</head>
<body>
{nav}
<header>
<h1>{title}</h1>
<p class="author">Автор: {author} <span class="meta">· {lang} · EPUB 3.2</span></p>
<a class="shelf-link" href="/view3d?book={id}">🧊 дивитись у 3D</a> · <a class="shelf-link" href="/books">☷ усі книги</a>
</header>

<section class="frame book">
<h2 class="frame-title">В книзі · {chap} розділів</h2>
<p class="frame-note">Нижче — те, що фізично входить до файлу <em>{epub_name}</em>.</p>
<ol class="toc">
{toc}
</ol>
</section>

<section class="frame site">
<h2 class="frame-title">Окремо на сайті · не в книзі</h2>
<p class="frame-note">Це інструменти просмоторщика — вони не потрапляють у EPUB і не є частиною книги.</p>
<div class="kdp">
<p class="badge {badge_cls}">{badge_out} — {passes}/{total} перевірок KDP</p>
<table class="report">
<thead><tr><th>#</th><th>Правило</th><th>Деталь</th></tr></thead>
<tbody>
{rows}
</tbody>
</table>
<p class="links"><a href="/{id}/check">Повний звіт /check</a> · <a href="/{id}/chapter/1">Почати книгу</a></p>
</div>
</section>
</body>
</html>
"#,
        title = html_esc(&book.title),
        author = html_esc(&book.author),
        lang = html_esc(&book.language),
        id = book_id,
        nav = nav_html("/"),
        epub_name = html_esc("*.epub"),
        badge_out = badge_out,
        badge_cls = badge_cls,
        passes = report.passes(),
        total = report.items.len(),
        rows = rows,
        chap = chapters.len(),
        toc = toc,
    )
}

/// One chapter of interior HTML for the virtual stand (RB-48).
#[derive(Debug, Clone, serde::Serialize)]
pub struct InteriorChapter {
    pub number: u32,
    pub title: String,
    pub html: String,
}

/// Chapter bodies from the EPUB, in spine/`book.json` order.
pub fn interior_chapters(epub: &Epub, book: &Book) -> Vec<InteriorChapter> {
    book.chapters
        .iter()
        .map(|meta| {
            let path = format!("OEBPS/chapter-{:02}.xhtml", meta.number);
            let raw = epub.text(&path).unwrap_or_default();
            let html = extract_body(&raw).unwrap_or(raw);
            InteriorChapter {
                number: meta.number,
                title: meta.title.clone(),
                html,
            }
        })
        .collect()
}

/// Extract the inner `<body>` content of an XHTML page.
fn extract_body(xhtml: &str) -> Option<String> {
    let low = xhtml.to_ascii_lowercase();
    let start = low.find("<body")?;
    let body_open_end = xhtml[start..].find('>')? + start + 1;
    let closing = xhtml.rfind("</body>")?;
    Some(xhtml[body_open_end..closing].to_string())
}

/// Render a single chapter as clean KDP-acceptable XHTML.
fn render_chapter(epub: &Epub, book: &Book, num: &str, book_id: &str) -> Option<String> {
    // Find the matching chapter meta to reuse the exact title.
    let meta = book.chapters.iter().find(|c| c.number.to_string() == num)?;
    let path = format!("OEBPS/chapter-{:02}.xhtml", meta.number);
    let raw = epub.text(&path)?;
    let body = extract_body(&raw).unwrap_or_else(|| raw.clone());
    Some(format!(
        r#"<!DOCTYPE html>
<html lang="uk">
<head>
<meta charset="utf-8"/>
<title>{num_cls} — {title_esc}</title>
<link rel="stylesheet" href="/styles.css"/>
</head>
<body>
<nav class="chapnav">
<a href="/{id}/">← Повернутись до книги</a>
</nav>
{body}
</body>
</html>
"#,
        num_cls = num,
        title_esc = html_esc(&meta.title),
        id = book_id,
        body = body,
    ))
}

/// Render the shelf page listing every discovered book.
fn render_shelf(books: &[LoadedBook]) -> String {
    let mut cards = String::new();
    for b in books {
        let n = b.book.chapters.len();
        let words: usize = n; // chapter count shown; word totals load on open
        cards.push_str(&format!(
            "<div class=\"book-card\">\n\
             \x20 <h3><a href=\"/{id}/\">{title}</a></h3>\n\
             \x20 <p class=\"author\">{author} <span class=\"langbadge lb-{lc}\">{lang}</span> <span class=\"meta\">· {n} розд. · ~{w}</span></p>\n\
             \x20 <p class=\"meta\">{path}</p>\n\
             \x20 <p><a class=\"btn\" href=\"/{id}/chapter/1\">Читати</a> \
             \x20 <a class=\"btn\" href=\"/{id}/check\">KDP-перевірка</a> \
             \x20 <a class=\"btn\" style=\"border-color:#7aa2f7\" href=\"/view3d?book={id}\">🧊 3D</a></p>\n\
             </div>\n",
            id = b.id,
            title = html_esc(&b.book.title),
            author = html_esc(&b.book.author),
            lang = html_esc(&b.book.language),
            lc = html_esc(&b.book.language),
            n = n,
            w = words,
            path = html_esc(&b.path),
        ));
    }
    format!(
        r#"<!DOCTYPE html>
<html lang="uk">
<head>
<meta charset="utf-8"/>
<title>Книжкова полиця — EPUB просмоторщик</title>
<link rel="stylesheet" href="/styles.css"/>
<style>
 .langbadge{{display:inline-block;padding:.05em .5em;border-radius:6px;font-size:.75em;font-weight:600;vertical-align:middle}}
 .lb-uk{{background:#2e7d32;color:#fff}} .lb-en{{background:#1565c0;color:#fff}}
  .themeChip{{position:fixed;top:3.1em;right:.8em;z-index:5;background:var(--panel,#232734);color:var(--text,#e6e6e6);border:1px solid var(--line,#2f3542);border-radius:8px;padding:.25em .7em;cursor:pointer;font:inherit}}
</style>
</head>
<body>
{nav}
<button class="themeChip" id="themeChip">◐</button>
<header>
<h1>Книжкова полиця</h1>
<p class="author">Знайдено EPUB: <span class="meta">{count}</span></p>
</header>
{cards}
<script>
(function(){{var L=localStorage;var s=L.getItem('rb.shelfTheme')||'dark';
 function apply(){{document.documentElement.style.setProperty('--bg',s==='light'?'#f5f5f4':'#12141a');document.documentElement.style.setProperty('--panel',s==='light'?'#fff':'#232734');document.documentElement.style.setProperty('--text',s==='light'?'#1f2328':'#e6e6e6');document.documentElement.style.setProperty('--line',s==='light'?'#d8d4cd':'#2f3542');document.body.style.background=s==='light'?'#f5f5f4':'';document.querySelectorAll('.frame,.book-card').forEach(function(e){{e.style.background=s==='light'?'#fff':''}});document.querySelectorAll('body,h1,h2,h3,p,li,td').forEach(function(e){{e.style.color=s==='light'?'#1f2328':''}})}}
 document.getElementById('themeChip').onclick=function(){{s=s==='dark'?'light':'dark';L.setItem('rb.shelfTheme',s);apply()}};apply();}})();
</script>
</body>
</html>
"#,
        count = books.len(),
        cards = cards,
        nav = nav_html("/"),
    )
}

/// The served CSS: EPUB's styles.minified plus a little viewer chrome.
fn viewer_css(epub: &Epub) -> String {
    let base = epub.text("OEBPS/styles.css").unwrap_or_default();
    let chrome = r#"
 /* viewer chrome — dark theme */
 .rb-nav{position:sticky;top:0;z-index:7;display:flex;gap:.9em;align-items:center;padding:.5em .9em;background:#161922;border-bottom:1px solid #2f3542;font:600 14px/1.4 system-ui,'Segoe UI',sans-serif}
 .rb-nav .rb-brand{color:#7aa2f7;font-weight:800;letter-spacing:.04em}
 .rb-nav a{color:#9aa0ac;text-decoration:none;padding:.2em .1em;border-bottom:2px solid transparent}
 .rb-nav a:hover{color:#e6e6e6}
 .rb-nav a.active{color:#7aa2f7;border-bottom-color:#7aa2f7}
 .rb-nav .rb-home{margin-left:auto;font-weight:400;font-size:.85em}
 :root{
  --bg:#12141a; --panel:#1b1e27; --panel2:#232734;
  --text:#e6e6e6; --muted:#9aa0ac; --accent:#7aa2f7;
  --ok:#3fb950; --fail:#f85149; --line:#2f3542;
}
html,body{background:var(--bg);}
body{font-family:Georgia,"EB Garamond",serif;line-height:1.7;margin:4% 8%;color:var(--text);}
header h1{margin-bottom:0.1em;color:#fff;}
.author .meta{color:var(--muted);}
.frame{border:2px solid var(--line);border-radius:12px;padding:1.2em 1.6em;margin:1.4em 0;background:var(--panel);}
.frame.book{border-color:var(--accent);}
.frame-title{margin:0 0 0.2em;color:#fff;}
.frame-note{color:var(--muted);margin:0 0 0.9em;font-size:0.95em;}
.frame .toc{margin:0;padding-left:1.4em;}
.frame .toc li{margin:0.35em 0;}
.frame .toc a{color:var(--accent);text-decoration:none;}
.frame .toc a:hover{text-decoration:underline;}
.badge{display:inline-block;padding:0.3em 0.8em;border-radius:4px;font-weight:bold;}
.badge.pass{background:#14261a;color:var(--ok);border:1px solid var(--ok);}
.badge.fail{background:#2a1717;color:var(--fail);border:1px solid var(--fail);}
table.report{border-collapse:collapse;margin:1em 0;width:100%;}
table.report th,table.report td{border:1px solid var(--line);padding:0.4em 0.6em;text-align:left;color:var(--text);}
table.report th{background:var(--panel2);}
tr.ok td:first-child{color:var(--ok);}
tr.fail td:first-child{color:var(--fail);}
.links a{color:var(--accent);}
.chapnav{margin-bottom:1.4em;}
.chapnav a{color:var(--accent);text-decoration:none;}

/* dark-theme overrides for the EPUB's own light styles */
body, h1, h2, h3, h4 { color: var(--text); }
code {
  background: #2a2f3a;
  border: 1px solid #383f4d;
  color: #f0c674;
}
pre {
  background: #1a1f2a;
  border: 1px solid #383f4d;
  color: var(--text);
}
pre code { background: none; border: none; color: var(--text); }
blockquote {
  color: #c9d1d9;
  border-left: 3px solid var(--accent);
}
hr { border-top: 1px solid var(--line); }
a { color: var(--accent); }

/* shelf / book cards */
.shelf-link{font-size:0.85em;display:inline-block;margin-top:0.3em;color:var(--muted);text-decoration:none;}
.shelf-link:hover{color:var(--accent);}
.book-card{background:var(--panel);border:1px solid var(--line);border-left:4px solid var(--accent);border-radius:8px;padding:0.9em 1.2em;margin:1em 0;}
.book-card h3{margin:0 0 0.2em;color:#fff;}
.book-card h3 a{color:var(--accent);text-decoration:none;}
.book-card h3 a:hover{text-decoration:underline;}
.btn{display:inline-block;margin:0.4em 0.5em 0 0;padding:0.35em 0.9em;border:1px solid var(--accent);border-radius:5px;color:var(--accent);text-decoration:none;background:transparent;}
.btn:hover{background:var(--accent);color:var(--bg);}
"#;
    format!("{base}\n{chrome}")
}

/// Render the `/check` report page.
fn render_check(epub: &Epub, book: &Book) -> String {
    let report = kdp_check(epub, book);
    let passed = report.passed();
    let mut lines = String::new();
    lines.push_str(&format!(
        "{{\n  \"passed\": {},\n  \"passes\": {},\n  \"total\": {},\n  \"items\": [\n",
        passed,
        report.passes(),
        report.items.len()
    ));
    for (i, item) in report.items.iter().enumerate() {
        let comma = if i + 1 < report.items.len() { "," } else { "" };
        lines.push_str(&format!(
            "    {{\"name\": \"{n}\", \"ok\": {o}, \"detail\": \"{d}\"}}{c}\n",
            n = json_esc(&item.name),
            o = item.ok,
            d = json_esc(&item.detail),
            c = comma
        ));
    }
    lines.push_str("  ]\n}\n");
    lines
}

fn html_esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn json_esc(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

/// One EPUB discovered on disk, ready to preview. `id` is a stable URL slug.
#[derive(Debug, Clone)]
pub struct LoadedBook {
    pub id: String,
    pub path: String,
    pub book: Book,
    pub epub: Epub,
}

/// Scan `dir` (recursively) for `*.epub` files and load them for preview.
///
/// For a given EPUB we first look for a sibling/config `book.json` (in `dir`
/// or the same folder as the epub). If none exists we auto-parse the package
/// (OPF + spine) into a `Book`. This is the "just drop an epub and watch it"
/// behaviour.
pub fn discover_books(dir: &std::path::Path) -> Result<Vec<LoadedBook>, String> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let entries = std::fs::read_dir(&d).map_err(|e| format!("Немає доступу до {d:?}: {e}"))?;
        for ent in entries.flatten() {
            let p = ent.path();
            if p.is_dir() {
                // Never walk build/tooling dirs — their *.epub are test artifacts.
                let name = p.file_name().map(|s| s.to_string_lossy().into_owned());
                if matches!(
                    name.as_deref(),
                    Some(".git" | "target" | "node_modules" | ".cargo" | "products")
                ) {
                    continue;
                }
                stack.push(p);
                continue;
            }
            if p.extension().is_some_and(|e| e == "epub")
                && let Ok(some) = discover_one(&p, Some(dir))
            {
                out.push(some);
            }
        }
    }
    out.sort_by(|a, b| a.book.title.cmp(&b.book.title));
    // De-duplicate slugs: same title in different languages gets language
    // tags (`rust-uk`/`rust-en`); same title + language falls back to
    // `-2`, `-3` on top of the first occurrence.
    {
        let mut groups: std::collections::HashMap<String, Vec<usize>> =
            std::collections::HashMap::new();
        for (i, b) in out.iter().enumerate() {
            groups.entry(b.id.clone()).or_default().push(i);
        }
        for id in groups.keys() {
            let idxs = &groups[id];
            if idxs.len() < 2 {
                continue;
            }
            let langs: std::collections::HashSet<String> =
                idxs.iter().map(|&i| out[i].book.language.clone()).collect();
            if langs.len() == idxs.len() {
                for &i in idxs {
                    let base = out[i].id.clone();
                    let lang = out[i].book.language.clone();
                    out[i].id = format!("{base}-{lang}");
                }
            } else {
                for (n, &i) in idxs.iter().enumerate().skip(1) {
                    out[i].id = format!("{id}-{}", n + 1);
                }
            }
        }
    }
    Ok(out)
}

/// Load a single EPUB into a `LoadedBook`, preferring an adjacent `book.json`.
fn discover_one(
    path: &std::path::Path,
    root: Option<&std::path::Path>,
) -> Result<LoadedBook, String> {
    let epub = Epub::from_path(&path.to_string_lossy())?;
    let parent = path.parent();
    // Nearest `book.json` wins: walk up from the epub's folder toward the scan
    // root; the first existing file labels the book.
    let mut book = None;
    let mut dir = parent.map(|p| p.to_path_buf());
    while let Some(d) = dir {
        if let Ok(b) = crate::load_book(d.join("book.json")) {
            book = Some(b);
            break;
        }
        if root.is_some_and(|r| d == r.to_path_buf()) {
            break; // reached the scan root without a book.json
        }
        dir = d.parent().map(|p| p.to_path_buf());
    }
    let book = match book {
        Some(b) => b,
        None => epub.parse_opf_book()?,
    };
    let id = slug(&book.title);
    Ok(LoadedBook {
        id,
        path: path.to_string_lossy().into_owned(),
        book,
        epub,
    })
}

/// A filesystem-safe slug for a book title, used as a URL id.
pub(crate) fn slug(title: &str) -> String {
    let mut s = String::new();
    let mut last_dash = false;
    for c in title.chars() {
        if c.is_ascii_alphanumeric() {
            s.push(c.to_ascii_lowercase());
            last_dash = false;
        } else if matches!(c, ' ' | '-') && !last_dash {
            s.push('-');
            last_dash = true;
        }
    }
    let s = s.trim_matches('-').to_string();
    if s.is_empty() { "book".to_string() } else { s }
}

/// True when `s` is already a canonical slug (used to fence API path params).
pub(crate) fn slug_is_safe(s: &str) -> bool {
    !s.is_empty() && s.len() <= 64 && slug(s) == s
}

/// The book selected by a request path prefix, falling back to the first.
fn choose<'a>(books: &'a [LoadedBook], path: &'a str) -> (&'a LoadedBook, &'a str) {
    let mut rest = path;
    let mut book = &books[0];
    if let Some(rest_path) = path.strip_prefix('/') {
        let slash = rest_path.find('/');
        let first_seg = &rest_path[..slash.unwrap_or(rest_path.len())];
        if let Some(found) = books.iter().find(|b| b.id == first_seg) {
            book = found;
            rest = if let Some(sl) = slash {
                &rest_path[sl..]
            } else {
                "/"
            };
        }
    }
    (book, rest)
}

/// Decide the HTTP reply for a path. Returns (status, body, mime).
pub(crate) fn route(path: &str, books: &[LoadedBook]) -> (&'static str, String, &'static str) {
    if path == "/" || path.is_empty() {
        if books.len() == 1 {
            let b = &books[0];
            return (
                "200 OK",
                render_index(&b.epub, &b.book, &b.id),
                "text/html; charset=utf-8",
            );
        }
        return ("200 OK", render_shelf(books), "text/html; charset=utf-8");
    }
    if path == "/health" {
        return ("200 OK", "ok".to_string(), "text/plain; charset=utf-8");
    }
    if path == "/books" || path == "/" {
        return ("200 OK", render_shelf(books), "text/html; charset=utf-8");
    }

    // Resolve optional /{id} prefix; default to the first book.
    let (book, rest) = choose(books, path);
    if rest == "/" || rest.is_empty() {
        return (
            "200 OK",
            render_index(&book.epub, &book.book, &book.id),
            "text/html; charset=utf-8",
        );
    }
    if rest == "/check" {
        return (
            "200 OK",
            render_check(&book.epub, &book.book),
            "application/json; charset=utf-8",
        );
    }
    if rest == "/styles.css" {
        return ("200 OK", viewer_css(&book.epub), "text/css; charset=utf-8");
    }
    if rest
        .strip_prefix("/")
        .is_some_and(|r| r.starts_with("chapter/"))
    {
        let num = &rest[("/chapter/").len()..];
        return match render_chapter(&book.epub, &book.book, num, &book.id) {
            Some(page) => ("200 OK", page, "text/html; charset=utf-8"),
            None => (
                "404 Not Found",
                page_not_found(num),
                "text/html; charset=utf-8",
            ),
        };
    }
    (
        "404 Not Found",
        page_not_found(path),
        "text/html; charset=utf-8",
    )
}

fn page_not_found(what: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="uk"><head><meta charset="utf-8"/><title>404</title>
<link rel="stylesheet" href="/styles.css"/></head>
<body>
<h1>404 — не знайдено</h1>
<p>Стаття <code>{}</code> не існує. <a href="/">На зміст →</a></p>
</body></html>
"#,
        html_esc(what)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Book, ChapterMeta};

    /// Build an in-memory Epub that mirrors the real built book's skeleton.
    fn build_test_epub() -> Epub {
        let mut entries = vec![
            ("mimetype".to_string(), b"application/epub+xml".to_vec()),
            (
                "META-INF/container.xml".to_string(),
                br#"<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container"><rootfiles><rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/></rootfiles></container>"#.to_vec(),
            ),
            (
                "OEBPS/content.opf".to_string(),
                br#"<package><metadata xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:title>T</dc:title><dc:creator>A</dc:creator><dc:language>uk</dc:language><dc:identifier id="book-id">urn:uuid:1234</dc:identifier><dc:date>2026-01-01</dc:date><meta property="dcterms:modified">2026-01-01T00:00:00Z</meta></metadata><spine><itemref idref="ch01"/></spine></package>"#.to_vec(),
            ),
            (
                "OEBPS/nav.xhtml".to_string(),
                "<nav epub:type=\"toc\"><li><a href=\"chapter-01.xhtml\">Розділ 1</a></li></nav>"
                    .as_bytes()
                    .to_vec(),
            ),
            ("OEBPS/styles.css".to_string(), b"body { color: black; }".to_vec()),
            (
                "OEBPS/chapter-01.xhtml".to_string(),
                "<html><body><h1>Перша</h1><p>Спокій.</p></body></html>".as_bytes().to_vec(),
            ),
        ];
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        // ensure mimetype is first, as in a real EPUB
        entries.retain(|(n, _)| n != "mimetype");
        entries.insert(
            0,
            ("mimetype".to_string(), b"application/epub+xml".to_vec()),
        );
        Epub { entries }
    }

    fn test_book() -> Book {
        Book {
            title: "Тест".to_string(),
            author: "Автор".to_string(),
            edition: 1,
            year: 2026,
            format: "EPUB 3.2".to_string(),
            language: "uk".to_string(),
            isbn: None,
            chapters: vec![ChapterMeta {
                number: 1,
                title: "Перша".to_string(),
                file: "chapters/01.md".to_string(),
            }],
        }
    }

    #[test]
    fn epub_get_and_text() {
        let e = build_test_epub();
        assert!(e.get("mimetype").is_some());
        assert_eq!(
            e.text("OEBPS/nav.xhtml").unwrap(),
            "<nav epub:type=\"toc\"><li><a href=\"chapter-01.xhtml\">Розділ 1</a></li></nav>"
        );
        assert!(e.get("absent").is_none());
    }

    #[test]
    fn kdp_check_passes_on_clean_book() {
        let e = build_test_epub();
        let report = kdp_check(&e, &test_book());
        assert!(report.passed(), "report should pass:\n{:?}", report.items);
    }

    #[test]
    fn kdp_check_flags_en_missing_isbn() {
        let e = build_test_epub();
        let mut book = test_book();
        book.language = "en".to_string();
        let report = kdp_check(&e, &book);
        let item = report.items.iter().find(|i| i.name == "opf:isbn").unwrap();
        assert!(!item.ok, "EN without urn:isbn must fail: {item:?}");
        book.isbn = Some("978-3-16-148410-0".to_string());
        let report = kdp_check(&e, &book);
        let item = report.items.iter().find(|i| i.name == "opf:isbn").unwrap();
        assert!(!item.ok, "declared ISBN missing from OPF: {item:?}");
    }

    #[test]
    fn kdp_check_en_isbn_in_opf_ok() {
        let mut e = build_test_epub();
        e.entries
            .iter_mut()
            .filter(|(n, _)| n == "OEBPS/content.opf")
            .for_each(|(_, b)| {
                let s = String::from_utf8_lossy(b);
                let s = s.replace(
                    "</metadata>",
                    "<dc:identifier id=\"pub-id\">urn:isbn:9783161484100</dc:identifier></metadata>",
                );
                *b = s.into_bytes();
            });
        let mut book = test_book();
        book.language = "en".to_string();
        book.isbn = Some("978-3-16-148410-0".to_string());
        let report = kdp_check(&e, &book);
        let item = report.items.iter().find(|i| i.name == "opf:isbn").unwrap();
        assert!(item.ok, "{item:?}");
    }

    #[test]
    fn kdp_check_flags_forbidden_script() {
        let mut e = build_test_epub();
        e.entries
            .iter_mut()
            .filter(|(n, _)| n == "OEBPS/chapter-01.xhtml")
            .for_each(|(_, b)| *b = b"<script>alert(1)</script><body>x</body>".to_vec());
        let report = kdp_check(&e, &test_book());
        assert!(!report.passed());
        let forbidden = report
            .items
            .iter()
            .find(|i| i.name == "xhtml:forbidden")
            .unwrap();
        assert!(!forbidden.ok);
        assert!(forbidden.detail.contains("<script"));
    }

    #[test]
    fn kdp_check_flags_raw_ampersand() {
        let mut e = build_test_epub();
        e.entries
            .iter_mut()
            .filter(|(n, _)| n == "OEBPS/chapter-01.xhtml")
            .for_each(|(_, b)| *b = b"<body>and this & that</body>".to_vec());
        let report = kdp_check(&e, &test_book());
        let forbidden = report
            .items
            .iter()
            .find(|i| i.name == "xhtml:forbidden")
            .unwrap();
        assert!(!forbidden.ok);
    }

    #[test]
    fn raw_ampersand_detection() {
        assert!(raw_ampersand("a & b"));
        assert!(!raw_ampersand("a &amp; b"));
        assert!(!raw_ampersand("&lt;&gt;&quot;"));
    }

    #[test]
    fn route_serves_index_check_css_chapter_404() {
        let e = build_test_epub();
        let book = test_book();
        let books = vec![LoadedBook {
            id: "test".to_string(),
            path: "build/rust_book.epub".to_string(),
            book,
            epub: e,
        }];

        let (s, body, mime) = route("/", &books);
        assert_eq!(s, "200 OK");
        assert!(body.contains("В книзі"));
        assert!(body.contains("Окремо на сайті"));
        assert!(body.contains("перевірок KDP"));
        assert!(mime.starts_with("text/html"));

        let (s, body, _) = route("/test/chapter/1", &books);
        assert_eq!(s, "200 OK");
        assert!(body.contains("<h1>Перша</h1>"));

        let (s, body, _) = route("/chapter/1", &books);
        assert_eq!(s, "200 OK");
        assert!(body.contains("<h1>Перша</h1>"));

        let (s, _, m) = route("/test/check", &books);
        assert_eq!(s, "200 OK");
        assert!(m.starts_with("application/json"));

        let (s, _, _) = route("/styles.css", &books);
        assert_eq!(s, "200 OK");

        let (s, _, _) = route("/nope", &books);
        assert_eq!(s, "404 Not Found");
    }

    #[test]
    fn single_book_root_shows_index_multiple_show_shelf() {
        let single = vec![LoadedBook {
            id: "a".to_string(),
            path: "a.epub".to_string(),
            book: test_book(),
            epub: build_test_epub(),
        }];
        let (s, body, _) = route("/", &single);
        assert_eq!(s, "200 OK");
        assert!(body.contains("В книзі"));
        assert!(!body.contains("Книжкова полиця"));

        let two = vec![
            LoadedBook {
                id: "a".to_string(),
                path: "a.epub".to_string(),
                book: test_book(),
                epub: build_test_epub(),
            },
            LoadedBook {
                id: "b".to_string(),
                path: "b.epub".to_string(),
                book: test_book(),
                epub: build_test_epub(),
            },
        ];
        let (s, body, _) = route("/", &two);
        assert_eq!(s, "200 OK");
        assert!(body.contains("Книжкова полиця"));
    }

    #[test]
    fn parse_opf_book_reads_metadata_and_spine() {
        let mut e = build_test_epub();
        // give the OPF a real manifest + two spine items so auto-parse works
        e.entries
            .iter_mut()
            .filter(|(n, _)| n == "OEBPS/content.opf")
            .for_each(|(_, b)| {
                *b = r#"<package><metadata xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:title>Моя книга</dc:title><dc:creator>Хтось</dc:creator><dc:language>uk</dc:language></metadata><manifest><item id="ch01" href="chapter-01.xhtml" media-type="application/xhtml+xml"/></manifest><spine><itemref idref="ch01"/></spine></package>"#.as_bytes().to_vec()
            });
        let b = e.parse_opf_book().unwrap();
        assert_eq!(b.title, "Моя книга");
        assert_eq!(b.author, "Хтось");
        assert_eq!(b.language, "uk");
        assert_eq!(b.chapters.len(), 1);
        assert_eq!(b.chapters[0].title, "Перша");
        assert_eq!(b.chapters[0].file, "OEBPS/chapter-01.xhtml");
    }

    #[test]
    fn slug_is_lowercase_and_dashes() {
        assert_eq!(slug("Rust Book"), "rust-book");
        assert_eq!(slug("Rust перед сном"), "rust");
        assert_eq!(slug("   "), "book");
    }

    #[test]
    fn choose_selects_book_by_prefix() {
        let two = vec![
            LoadedBook {
                id: "first".to_string(),
                path: "a.epub".to_string(),
                book: test_book(),
                epub: build_test_epub(),
            },
            LoadedBook {
                id: "second".to_string(),
                path: "b.epub".to_string(),
                book: test_book(),
                epub: build_test_epub(),
            },
        ];
        let (b, rest) = choose(&two, "/second/chapter/3");
        assert_eq!(b.id, "second");
        assert_eq!(rest, "/chapter/3");
        let (b, rest) = choose(&two, "/chapter/1");
        assert_eq!(b.id, "first");
        assert_eq!(rest, "/chapter/1");
    }

    #[test]
    fn extract_body_returns_inner_content() {
        let body = extract_body("<html><body><h1>Хай</h1><p>текст</p></body></html>").unwrap();
        assert_eq!(body, "<h1>Хай</h1><p>текст</p>");
    }

    #[test]
    fn interior_chapters_uses_body_inner_html() {
        let ch = interior_chapters(&build_test_epub(), &test_book());
        assert_eq!(ch.len(), 1);
        assert_eq!(ch[0].number, 1);
        assert!(ch[0].html.contains("<h1>Перша</h1>"));
        assert!(!ch[0].html.to_ascii_lowercase().contains("<body"));
    }

    #[test]
    fn nav_html_has_gsv_tips_and_i18n_attrs() {
        let n = nav_html("/studio");
        assert!(n.contains("data-tip="));
        assert!(n.contains("data-uk=\"Studio\""));
        assert!(n.contains("data-en=\"Shelf\""));
        assert!(n.contains("id=\"rbLangChip\""));
        assert!(n.contains("id=\"rbTip\""));
        assert!(n.contains("DOMContentLoaded"));
        assert!(n.contains("class=\"active\""));
        assert!(n.contains("href=\"/studio\""));
        assert!(n.contains("rbBindBoxFs"));
        assert!(n.contains(".rb-box.fullscreen"));
        assert!(n.contains("rb-box-fs"));
    }

    #[test]
    fn external_urls_found_but_namespaces_ignored() {
        let html = r#"<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops">
        <a href="http://example.com/x">link</a>
        <img src="https://cdn.example/i.png"/>
        <a href="page.xhtml">internal</a>
        </html>"#;
        let found = external_resource_urls(html);
        assert!(found.iter().any(|u| u.starts_with("http://example.com")));
        assert!(found.iter().any(|u| u.starts_with("https://cdn.example")));
        assert_eq!(found.len(), 2);
    }

    #[test]
    fn external_urls_terminates_on_mixed_content() {
        // Regression: this input must not loop forever.
        let html = r#"<p class="x">текст &amp; більше; <em>з *зірочками*</em></p><div></div>"#;
        let found = external_resource_urls(html);
        assert!(found.is_empty());
    }
}
