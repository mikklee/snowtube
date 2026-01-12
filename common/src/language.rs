//! Language options for video platforms

use std::fmt;
use std::sync::OnceLock;

/// A language option with its corresponding locale codes
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageOption {
    /// Display name in native script (e.g., "日本語", "한국어")
    pub name: &'static str,
    /// English name (e.g., "Japanese", "Korean")
    pub english_name: &'static str,
    /// Language code (hl parameter, e.g., "ja", "ko")
    pub hl: &'static str,
    /// Region code (gl parameter, e.g., "JP", "KR")
    pub gl: &'static str,
}

impl fmt::Display for LanguageOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.name == self.english_name {
            write!(f, "{}", self.name)
        } else {
            write!(f, "{} ({})", self.name, self.english_name)
        }
    }
}

/// Cached list of all languages
static ALL_LANGUAGES: OnceLock<Vec<LanguageOption>> = OnceLock::new();

/// Get all available language options sorted alphabetically
pub fn get_all_languages() -> &'static [LanguageOption] {
    ALL_LANGUAGES.get_or_init(|| {
        let mut languages = vec![
            // East Asian languages
            LanguageOption {
                name: "中文",
                english_name: "Chinese",
                hl: "zh-CN",
                gl: "CN",
            },
            LanguageOption {
                name: "日本語",
                english_name: "Japanese",
                hl: "ja",
                gl: "JP",
            },
            LanguageOption {
                name: "한국어",
                english_name: "Korean",
                hl: "ko",
                gl: "KR",
            },
            // European languages
            LanguageOption {
                name: "Shqip",
                english_name: "Albanian",
                hl: "sq",
                gl: "AL",
            },
            LanguageOption {
                name: "Euskara",
                english_name: "Basque",
                hl: "eu",
                gl: "ES",
            },
            LanguageOption {
                name: "Беларуская",
                english_name: "Belarusian",
                hl: "be",
                gl: "BY",
            },
            LanguageOption {
                name: "Bosanski",
                english_name: "Bosnian",
                hl: "bs",
                gl: "BA",
            },
            LanguageOption {
                name: "Български",
                english_name: "Bulgarian",
                hl: "bg",
                gl: "BG",
            },
            LanguageOption {
                name: "Català",
                english_name: "Catalan",
                hl: "ca",
                gl: "ES",
            },
            LanguageOption {
                name: "Hrvatski",
                english_name: "Croatian",
                hl: "hr",
                gl: "HR",
            },
            LanguageOption {
                name: "Čeština",
                english_name: "Czech",
                hl: "cs",
                gl: "CZ",
            },
            LanguageOption {
                name: "Dansk",
                english_name: "Danish",
                hl: "da",
                gl: "DK",
            },
            LanguageOption {
                name: "Nederlands",
                english_name: "Dutch",
                hl: "nl",
                gl: "NL",
            },
            LanguageOption {
                name: "English",
                english_name: "English",
                hl: "en",
                gl: "US",
            },
            LanguageOption {
                name: "Eesti",
                english_name: "Estonian",
                hl: "et",
                gl: "EE",
            },
            LanguageOption {
                name: "Suomi",
                english_name: "Finnish",
                hl: "fi",
                gl: "FI",
            },
            LanguageOption {
                name: "Français",
                english_name: "French",
                hl: "fr",
                gl: "FR",
            },
            LanguageOption {
                name: "Deutsch",
                english_name: "German",
                hl: "de",
                gl: "DE",
            },
            LanguageOption {
                name: "Ελληνικά",
                english_name: "Greek",
                hl: "el",
                gl: "GR",
            },
            LanguageOption {
                name: "Magyar",
                english_name: "Hungarian",
                hl: "hu",
                gl: "HU",
            },
            LanguageOption {
                name: "Íslenska",
                english_name: "Icelandic",
                hl: "is",
                gl: "IS",
            },
            LanguageOption {
                name: "Gaeilge",
                english_name: "Irish",
                hl: "en",
                gl: "IE",
            },
            LanguageOption {
                name: "Italiano",
                english_name: "Italian",
                hl: "it",
                gl: "IT",
            },
            LanguageOption {
                name: "Latviešu",
                english_name: "Latvian",
                hl: "lv",
                gl: "LV",
            },
            LanguageOption {
                name: "Lietuvių",
                english_name: "Lithuanian",
                hl: "lt",
                gl: "LT",
            },
            LanguageOption {
                name: "Македонски",
                english_name: "Macedonian",
                hl: "mk",
                gl: "MK",
            },
            LanguageOption {
                name: "Norsk",
                english_name: "Norwegian",
                hl: "no",
                gl: "NO",
            },
            LanguageOption {
                name: "Polski",
                english_name: "Polish",
                hl: "pl",
                gl: "PL",
            },
            LanguageOption {
                name: "Português",
                english_name: "Portuguese",
                hl: "pt",
                gl: "BR",
            },
            LanguageOption {
                name: "Română",
                english_name: "Romanian",
                hl: "ro",
                gl: "RO",
            },
            LanguageOption {
                name: "Русский",
                english_name: "Russian",
                hl: "ru",
                gl: "RU",
            },
            LanguageOption {
                name: "Српски",
                english_name: "Serbian",
                hl: "sr",
                gl: "RS",
            },
            LanguageOption {
                name: "Slovenčina",
                english_name: "Slovak",
                hl: "sk",
                gl: "SK",
            },
            LanguageOption {
                name: "Slovenščina",
                english_name: "Slovene",
                hl: "sl",
                gl: "SI",
            },
            LanguageOption {
                name: "Español",
                english_name: "Spanish",
                hl: "es",
                gl: "ES",
            },
            LanguageOption {
                name: "Svenska",
                english_name: "Swedish",
                hl: "sv",
                gl: "SE",
            },
            LanguageOption {
                name: "Українська",
                english_name: "Ukrainian",
                hl: "uk",
                gl: "UA",
            },
            LanguageOption {
                name: "Cymraeg",
                english_name: "Welsh",
                hl: "en",
                gl: "GB",
            },
            // Middle Eastern languages
            LanguageOption {
                name: "العربية",
                english_name: "Arabic",
                hl: "ar",
                gl: "SA",
            },
            LanguageOption {
                name: "עברית",
                english_name: "Hebrew",
                hl: "he",
                gl: "IL",
            },
            LanguageOption {
                name: "فارسی",
                english_name: "Persian",
                hl: "fa",
                gl: "IR",
            },
            LanguageOption {
                name: "Türkçe",
                english_name: "Turkish",
                hl: "tr",
                gl: "TR",
            },
            // South Asian languages
            LanguageOption {
                name: "বাংলা",
                english_name: "Bengali",
                hl: "bn",
                gl: "BD",
            },
            LanguageOption {
                name: "ગુજરાતી",
                english_name: "Gujarati",
                hl: "gu",
                gl: "IN",
            },
            LanguageOption {
                name: "हिन्दी",
                english_name: "Hindi",
                hl: "hi",
                gl: "IN",
            },
            LanguageOption {
                name: "मराठी",
                english_name: "Marathi",
                hl: "mr",
                gl: "IN",
            },
            LanguageOption {
                name: "ਪੰਜਾਬੀ",
                english_name: "Punjabi",
                hl: "pa",
                gl: "IN",
            },
            LanguageOption {
                name: "தமிழ்",
                english_name: "Tamil",
                hl: "ta",
                gl: "IN",
            },
            LanguageOption {
                name: "తెలుగు",
                english_name: "Telugu",
                hl: "te",
                gl: "IN",
            },
            LanguageOption {
                name: "اردو",
                english_name: "Urdu",
                hl: "ur",
                gl: "PK",
            },
            // Southeast Asian languages
            LanguageOption {
                name: "Bahasa Indonesia",
                english_name: "Indonesian",
                hl: "id",
                gl: "ID",
            },
            LanguageOption {
                name: "Bahasa Melayu",
                english_name: "Malay",
                hl: "ms",
                gl: "MY",
            },
            LanguageOption {
                name: "Tagalog",
                english_name: "Tagalog",
                hl: "tl",
                gl: "PH",
            },
            LanguageOption {
                name: "ไทย",
                english_name: "Thai",
                hl: "th",
                gl: "TH",
            },
            LanguageOption {
                name: "Tiếng Việt",
                english_name: "Vietnamese",
                hl: "vi",
                gl: "VN",
            },
            // Central Asian languages
            LanguageOption {
                name: "Қазақ",
                english_name: "Kazakh",
                hl: "kk",
                gl: "KZ",
            },
            LanguageOption {
                name: "Монгол",
                english_name: "Mongolian",
                hl: "mn",
                gl: "MN",
            },
            // Caucasian languages
            LanguageOption {
                name: "Հdelays",
                english_name: "Armenian",
                hl: "hy",
                gl: "AM",
            },
            LanguageOption {
                name: "Azərbaycan",
                english_name: "Azerbaijani",
                hl: "az",
                gl: "AZ",
            },
            LanguageOption {
                name: "ქართული",
                english_name: "Georgian",
                hl: "ka",
                gl: "GE",
            },
            // African languages
            LanguageOption {
                name: "Afrikaans",
                english_name: "Afrikaans",
                hl: "af",
                gl: "ZA",
            },
            LanguageOption {
                name: "Kiswahili",
                english_name: "Swahili",
                hl: "sw",
                gl: "KE",
            },
        ];

        // Sort alphabetically by English name for consistency
        languages.sort_by(|a, b| a.english_name.cmp(b.english_name));
        languages
    })
}

/// Find a language option by its locale codes (hl, gl)
pub fn get_language_by_locale(hl: &str, gl: &str) -> Option<&'static LanguageOption> {
    get_all_languages()
        .iter()
        .find(|lang| lang.hl == hl && lang.gl == gl)
}

/// Get the default language (English)
pub fn default_language() -> &'static LanguageOption {
    get_all_languages()
        .iter()
        .find(|lang| lang.hl == "en" && lang.gl == "US")
        .expect("English should always be available")
}

/// Default locale (English, US)
pub fn default_locale() -> (String, String) {
    ("en".to_string(), "US".to_string())
}
