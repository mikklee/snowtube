//! Multi-language relative time parsing for YouTube published_text fields
//!
//! Uses a word-based trie to parse strings like "2 days ago", "vor 3 Tagen", "1 il öncə" into minutes.

use std::collections::HashMap;
use std::sync::OnceLock;

/// Time unit multipliers (in minutes)
const SECONDS: u64 = 0; // rounds to 0 for sorting (very recent)
const MINUTES: u64 = 1;
const HOURS: u64 = 60;
const DAYS: u64 = 60 * 24;
const WEEKS: u64 = 60 * 24 * 7;
const MONTHS: u64 = 60 * 24 * 30;
const YEARS: u64 = 60 * 24 * 365;

/// A node in the word-based trie
#[derive(Default)]
struct TrieNode {
    /// If this node represents a complete match, the time multiplier
    value: Option<u64>,
    /// Children nodes keyed by the next word
    children: HashMap<&'static str, TrieNode>,
}

impl TrieNode {
    fn new() -> Self {
        Self::default()
    }

    fn with_value(value: u64) -> Self {
        Self {
            value: Some(value),
            children: HashMap::new(),
        }
    }

    /// Insert a word sequence into the trie
    fn insert(&mut self, words: &[&'static str], value: u64) {
        if words.is_empty() {
            return;
        }
        if words.len() == 1 {
            self.children
                .entry(words[0])
                .or_insert_with(|| TrieNode::with_value(value))
                .value = Some(value);
        } else {
            self.children
                .entry(words[0])
                .or_insert_with(TrieNode::new)
                .insert(&words[1..], value);
        }
    }

    /// Look up a word sequence, returning the value and number of words consumed
    fn lookup(&self, words: &[&str]) -> Option<(u64, usize)> {
        if words.is_empty() {
            return self.value.map(|v| (v, 0));
        }

        let word = words[0].to_lowercase();

        // Try to match this word
        if let Some(child) = self.children.get(word.as_str()) {
            // Try to match more words first (longest match)
            if let Some((value, consumed)) = child.lookup(&words[1..]) {
                return Some((value, consumed + 1));
            }
            // Otherwise return this node's value if it has one
            if let Some(value) = child.value {
                return Some((value, 1));
            }
        }

        // No match for this word, return our value if we have one
        self.value.map(|v| (v, 0))
    }
}

static TRIE: OnceLock<TrieNode> = OnceLock::new();

fn get_trie() -> &'static TrieNode {
    TRIE.get_or_init(|| {
        let mut root = TrieNode::new();

        // Helper macro to insert single words
        macro_rules! word {
            ($w:expr, $v:expr) => {
                root.insert(&[$w], $v);
            };
        }

        // Helper macro to insert word sequences (for disambiguation)
        macro_rules! words {
            ($v:expr, $($w:expr),+) => {
                root.insert(&[$($w),+], $v);
            };
        }

        // English
        word!("second", SECONDS);
        word!("seconds", SECONDS);
        word!("minute", MINUTES);
        word!("minutes", MINUTES);
        word!("hour", HOURS);
        word!("hours", HOURS);
        word!("day", DAYS);
        word!("days", DAYS);
        word!("week", WEEKS);
        word!("weeks", WEEKS);
        word!("month", MONTHS);
        word!("months", MONTHS);
        word!("year", YEARS);
        word!("years", YEARS);

        // Afrikaans
        word!("sekonde", SECONDS);
        word!("uur", HOURS);
        word!("dag", DAYS);
        word!("dae", DAYS);
        word!("weke", WEEKS);
        word!("maand", MONTHS);
        word!("maande", MONTHS);
        word!("jaar", YEARS);

        // Azerbaijani - "il" needs next word "öncə" to disambiguate from French "il y a"
        word!("saniyə", SECONDS);
        word!("dəqiqə", MINUTES);
        word!("saat", HOURS);
        word!("gün", DAYS);
        word!("həftə", WEEKS);
        word!("ay", MONTHS);
        words!(YEARS, "il", "öncə"); // "1 il öncə" = 1 year ago

        // Indonesian
        word!("detik", MINUTES);
        word!("menit", MINUTES);
        word!("jam", HOURS);
        word!("hari", DAYS);
        word!("minggu", WEEKS);
        word!("bulan", MONTHS);
        word!("tahun", YEARS);

        // Malay
        word!("minit", MINUTES);

        // Bosnian/Croatian/Serbian Latin
        word!("sekundi", SECONDS);
        word!("sekunde", SECONDS);
        word!("minuta", MINUTES);
        word!("sata", HOURS);
        word!("sati", HOURS);
        word!("dana", DAYS);
        word!("dan", DAYS);
        word!("sedmice", WEEKS);
        word!("sedmica", WEEKS);
        word!("tjedna", WEEKS);
        word!("tjedan", WEEKS);
        word!("mjesec", MONTHS);
        word!("mjeseca", MONTHS);
        word!("mjeseci", MONTHS);
        word!("godinu", YEARS);
        word!("godine", YEARS);
        word!("godina", YEARS);

        // Catalan
        word!("segons", SECONDS);
        word!("segon", SECONDS);
        word!("minuts", MINUTES);
        word!("minut", MINUTES);
        word!("hores", HOURS);
        word!("hora", HOURS);
        word!("dies", DAYS);
        word!("dia", DAYS);
        word!("setmanes", WEEKS);
        word!("setmana", WEEKS);
        word!("mesos", MONTHS);
        word!("mes", MONTHS);
        word!("anys", YEARS);
        word!("any", YEARS);

        // Danish
        word!("sekunder", SECONDS);
        word!("sekund", SECONDS);
        word!("minutter", MINUTES);
        word!("timer", HOURS);
        word!("time", HOURS);
        word!("dage", DAYS);
        word!("døgn", DAYS);
        word!("uger", WEEKS);
        word!("uge", WEEKS);
        word!("måneder", MONTHS);
        word!("måned", MONTHS);
        word!("år", YEARS);

        // German
        word!("sekunden", SECONDS);
        word!("sekunde", SECONDS);
        word!("minuten", MINUTES);
        word!("minute", MINUTES);
        word!("stunden", HOURS);
        word!("stunde", HOURS);
        word!("tagen", DAYS);
        word!("tag", DAYS);
        word!("wochen", WEEKS);
        word!("woche", WEEKS);
        word!("monaten", MONTHS);
        word!("monat", MONTHS);
        word!("jahren", YEARS);
        word!("jahr", YEARS);

        // Estonian
        word!("sekundi", SECONDS);
        word!("sekundit", SECONDS);
        word!("minuti", MINUTES);
        word!("minutit", MINUTES);
        word!("tunni", HOURS);
        word!("tundi", HOURS);
        word!("päeva", DAYS);
        word!("nädala", WEEKS);
        word!("nädalat", WEEKS);
        word!("kuu", MONTHS);
        word!("aasta", YEARS);

        // Spanish
        word!("segundos", SECONDS);
        word!("segundo", SECONDS);
        word!("minutos", MINUTES);
        word!("minuto", MINUTES);
        word!("horas", HOURS);
        word!("días", DAYS);
        word!("día", DAYS);
        word!("semanas", WEEKS);
        word!("semana", WEEKS);
        word!("meses", MONTHS);
        word!("años", YEARS);
        word!("año", YEARS);

        // Basque
        word!("minutu", MINUTES);
        word!("ordu", HOURS);
        word!("egun", DAYS);
        word!("aste", WEEKS);
        word!("hilabete", MONTHS);
        word!("urte", YEARS);

        // French
        word!("secondes", SECONDS);
        word!("seconde", SECONDS);
        word!("heures", HOURS);
        word!("heure", HOURS);
        word!("jours", DAYS);
        word!("jour", DAYS);
        word!("semaines", WEEKS);
        word!("semaine", WEEKS);
        word!("mois", MONTHS);
        word!("ans", YEARS);
        word!("an", YEARS);

        // Italian
        word!("secondi", SECONDS);
        word!("ore", HOURS);
        word!("ora", HOURS);
        word!("giorni", DAYS);
        word!("giorno", DAYS);
        word!("settimane", WEEKS);
        word!("settimana", WEEKS);
        word!("mesi", MONTHS);
        word!("mese", MONTHS);
        word!("anni", YEARS);
        word!("anno", YEARS);

        // Swahili
        word!("dakika", MINUTES);
        word!("saa", HOURS);
        word!("siku", DAYS);
        word!("wiki", WEEKS);
        word!("mwezi", MONTHS);
        word!("miezi", MONTHS);
        word!("mwaka", YEARS);
        word!("miaka", YEARS);

        // Latvian
        word!("sekundēm", SECONDS);
        word!("sekundes", SECONDS);
        word!("minūtēm", MINUTES);
        word!("minūtes", MINUTES);
        word!("stundām", HOURS);
        word!("stundas", HOURS);
        word!("dienām", DAYS);
        word!("nedēļām", WEEKS);
        word!("nedēļas", WEEKS);
        word!("mēnešiem", MONTHS);
        word!("mēneša", MONTHS);
        word!("gadiem", YEARS);
        word!("gada", YEARS);

        // Lithuanian
        word!("sekundžių", SECONDS);
        word!("minučių", MINUTES);
        word!("valandų", HOURS);
        word!("valandas", HOURS);
        word!("dienas", DAYS);
        word!("dienų", DAYS);
        word!("savaites", WEEKS);
        word!("savaičių", WEEKS);
        word!("mėnesius", MONTHS);
        word!("mėnesių", MONTHS);
        word!("mėnesį", MONTHS);
        word!("metus", YEARS);
        word!("metų", YEARS);

        // Hungarian
        word!("másodperce", SECONDS);
        word!("másodperccel", SECONDS);
        word!("perce", MINUTES);
        word!("perccel", MINUTES);
        word!("órája", HOURS);
        word!("órával", HOURS);
        word!("nappal", DAYS);
        word!("napja", DAYS);
        word!("héttel", WEEKS);
        word!("hete", WEEKS);
        word!("hónappal", MONTHS);
        word!("hónapja", MONTHS);
        word!("évvel", YEARS);
        word!("éve", YEARS);

        // Dutch
        word!("seconden", SECONDS);
        word!("minuut", MINUTES);
        word!("dagen", DAYS);
        word!("weken", WEEKS);
        word!("maanden", MONTHS);

        // Norwegian
        word!("minutt", MINUTES);
        word!("dager", DAYS);
        word!("uker", WEEKS);
        word!("uke", WEEKS);

        // Polish
        word!("sekund", SECONDS);
        word!("sekundy", SECONDS);
        word!("minut", MINUTES);
        word!("minuty", MINUTES);
        word!("godzin", HOURS);
        word!("godziny", HOURS);
        word!("dni", DAYS);
        word!("dzień", DAYS);
        word!("tygodnie", WEEKS);
        word!("tygodni", WEEKS);
        word!("miesiące", MONTHS);
        word!("miesięcy", MONTHS);
        word!("miesiąc", MONTHS);
        word!("lata", YEARS);
        word!("lat", YEARS);
        word!("rok", YEARS);

        // Portuguese
        word!("dias", DAYS);
        word!("mês", MONTHS);
        word!("anos", YEARS);
        word!("ano", YEARS);

        // Romanian
        word!("secundă", SECONDS);
        word!("oră", HOURS);
        word!("zile", DAYS);
        word!("zi", DAYS);
        word!("săptămâni", WEEKS);
        word!("săptămână", WEEKS);
        word!("luni", MONTHS);
        word!("lună", MONTHS);
        word!("ani", YEARS);

        // Slovak
        word!("sekundami", SECONDS);
        word!("sekundu", SECONDS);
        word!("minútami", MINUTES);
        word!("minútu", MINUTES);
        word!("hodinami", HOURS);
        word!("hodinou", HOURS);
        word!("dňami", DAYS);
        word!("dňom", DAYS);
        word!("týždňami", WEEKS);
        word!("týždňom", WEEKS);
        word!("mesiacmi", MONTHS);
        word!("mesiacom", MONTHS);
        word!("rokmi", YEARS);
        word!("rokom", YEARS);

        // Slovene
        word!("sekundo", SECONDS);
        word!("minutami", MINUTES);
        word!("urami", HOURS);
        word!("uro", HOURS);
        word!("dnevi", DAYS);
        word!("dnevom", DAYS);
        word!("tedni", WEEKS);
        word!("tednom", WEEKS);
        word!("meseci", MONTHS);
        word!("mesecem", MONTHS);
        word!("mesecema", MONTHS);
        word!("leti", YEARS);
        word!("letom", YEARS);

        // Finnish
        word!("sekuntia", SECONDS);
        word!("sekunti", SECONDS);
        word!("minuuttia", MINUTES);
        word!("minuutti", MINUTES);
        word!("tuntia", HOURS);
        word!("tunti", HOURS);
        word!("päivää", DAYS);
        word!("päivä", DAYS);
        word!("viikkoa", WEEKS);
        word!("viikko", WEEKS);
        word!("kuukautta", MONTHS);
        word!("kuukausi", MONTHS);
        word!("vuotta", YEARS);
        word!("vuosi", YEARS);

        // Swedish
        word!("minuter", MINUTES);
        word!("timmar", HOURS);
        word!("timme", HOURS);
        word!("dagar", DAYS);
        word!("veckor", WEEKS);
        word!("vecka", WEEKS);
        word!("månad", MONTHS);
        word!("månader", MONTHS);

        // Tagalog
        word!("oras", HOURS);
        word!("araw", DAYS);
        word!("linggo", WEEKS);
        word!("buwan", MONTHS);
        word!("taon", YEARS);

        // Vietnamese
        word!("giây", SECONDS);
        word!("phút", MINUTES);
        word!("giờ", HOURS);
        word!("ngày", DAYS);
        word!("tuần", WEEKS);
        word!("tháng", MONTHS);
        word!("năm", YEARS);

        // Turkish
        word!("saniye", SECONDS);
        word!("hafta", WEEKS);
        word!("yıl", YEARS);

        // Icelandic
        word!("sekúndum", SECONDS);
        word!("sekúndu", SECONDS);
        word!("mínútum", MINUTES);
        word!("mínútu", MINUTES);
        word!("klukkustundum", HOURS);
        word!("klukkustund", HOURS);
        word!("dögum", DAYS);
        word!("degi", DAYS);
        word!("vikum", WEEKS);
        word!("viku", WEEKS);
        word!("mánuðum", MONTHS);
        word!("mánuði", MONTHS);
        word!("árum", YEARS);
        word!("ári", YEARS);

        // Czech
        word!("sekundou", SECONDS);
        word!("minutou", MINUTES);
        word!("hodinou", HOURS);
        word!("dny", DAYS);
        word!("dnem", DAYS);
        word!("týdny", WEEKS);
        word!("týdnem", WEEKS);
        word!("měsíci", MONTHS);
        word!("měsícem", MONTHS);
        word!("rokem", YEARS);
        word!("lety", YEARS);

        // Greek
        word!("δευτερόλεπτα", SECONDS);
        word!("δευτερόλεπτο", SECONDS);
        word!("λεπτά", MINUTES);
        word!("λεπτό", MINUTES);
        word!("ώρες", HOURS);
        word!("ώρα", HOURS);
        word!("ημέρες", DAYS);
        word!("ημέρα", DAYS);
        word!("εβδομάδες", WEEKS);
        word!("εβδομάδα", WEEKS);
        word!("μήνες", MONTHS);
        word!("μήνα", MONTHS);
        word!("έτη", YEARS);
        word!("έτος", YEARS);
        word!("χρόνια", YEARS);
        word!("χρόνο", YEARS);

        // Belarusian
        word!("секунд", SECONDS);
        word!("секунды", SECONDS);
        word!("хвілін", MINUTES);
        word!("хвіліны", MINUTES);
        word!("гадзін", HOURS);
        word!("гадзіны", HOURS);
        word!("дзён", DAYS);
        word!("дні", DAYS);
        word!("тыдні", WEEKS);
        word!("тыдняў", WEEKS);
        word!("месяц", MONTHS);
        word!("месяцы", MONTHS);
        word!("месяцаў", MONTHS);
        word!("год", YEARS);
        word!("гады", YEARS);
        word!("гадоў", YEARS);

        // Bulgarian
        word!("секунди", SECONDS);
        word!("секунда", SECONDS);
        word!("минути", MINUTES);
        word!("минута", MINUTES);
        word!("часа", HOURS);
        word!("час", HOURS);
        word!("дни", DAYS);
        word!("ден", DAYS);
        word!("седмици", WEEKS);
        word!("седмица", WEEKS);
        word!("месец", MONTHS);
        word!("месеца", MONTHS);
        word!("години", YEARS);
        word!("година", YEARS);

        // Macedonian
        word!("дена", DAYS);
        word!("месеци", MONTHS);

        // Mongolian
        word!("секундын", SECONDS);
        word!("минутын", MINUTES);
        word!("цаг", HOURS);
        word!("цагийн", HOURS);
        word!("өдрийн", DAYS);
        word!("өдөр", DAYS);
        word!("хоногийн", DAYS);
        word!("долоо", WEEKS); // "долоо хоногийн"
        word!("сарын", MONTHS);
        word!("сар", MONTHS);
        word!("жилийн", YEARS);
        word!("жил", YEARS);

        // Russian
        word!("секунду", SECONDS);
        word!("минут", MINUTES);
        word!("минуты", MINUTES);
        word!("минуту", MINUTES);
        word!("часов", HOURS);
        word!("часа", HOURS);
        word!("дней", DAYS);
        word!("дня", DAYS);
        word!("день", DAYS);
        word!("недели", WEEKS);
        word!("недель", WEEKS);
        word!("неделю", WEEKS);
        word!("месяцев", MONTHS);
        word!("месяца", MONTHS);
        word!("лет", YEARS);
        word!("года", YEARS);

        // Serbian Cyrillic
        word!("секунде", SECONDS);
        word!("мину|те", MINUTES);
        word!("сата", HOURS);
        word!("сати", HOURS);
        word!("сат", HOURS);
        word!("дана", DAYS);
        word!("дан", DAYS);
        word!("недеље", WEEKS);
        word!("недеља", WEEKS);
        word!("месеца", MONTHS);
        word!("месеци", MONTHS);
        word!("године", YEARS);
        word!("година", YEARS);

        // Ukrainian
        word!("секунди", SECONDS);
        word!("хвилин", MINUTES);
        word!("хвилини", MINUTES);
        word!("хвилину", MINUTES);
        word!("годин", HOURS);
        word!("години", HOURS);
        word!("годину", HOURS);
        word!("днів", DAYS);
        word!("тижні", WEEKS);
        word!("тижнів", WEEKS);
        word!("тиждень", WEEKS);
        word!("місяців", MONTHS);
        word!("місяці", MONTHS);
        word!("місяць", MONTHS);
        word!("років", YEARS);
        word!("роки", YEARS);
        word!("рік", YEARS);

        // Kazakh
        word!("сағат", HOURS);
        word!("күн", DAYS);
        word!("апта", WEEKS);
        word!("ай", MONTHS);
        word!("жыл", YEARS);

        // Armenian
        word!("օր", DAYS);
        word!("շաբաթ", WEEKS);
        word!("ամիս", MONTHS);
        word!("տարի", YEARS);

        // Hebrew
        word!("שניות", SECONDS);
        word!("שנייה", SECONDS);
        word!("דקות", MINUTES);
        word!("דקה", MINUTES);
        word!("שעות", HOURS);
        word!("שעה", HOURS);
        word!("ימים", DAYS);
        word!("יום", DAYS);
        word!("שבועות", WEEKS);
        word!("שבוע", WEEKS);
        word!("חודשים", MONTHS);
        word!("חודש", MONTHS);
        word!("חודשיים", MONTHS);
        word!("שנים", YEARS);
        word!("שנה", YEARS);

        // Urdu
        word!("سیکنڈ", SECONDS);
        word!("منٹ", MINUTES);
        word!("گھنٹے", HOURS);
        word!("گھنٹہ", HOURS);
        word!("دنوں", DAYS);
        word!("دن", DAYS);
        word!("ہفتے", WEEKS);
        word!("ہفتہ", WEEKS);
        word!("مہینے", MONTHS);
        word!("مہینہ", MONTHS);
        word!("سال", YEARS);

        // Arabic
        word!("ثانية", SECONDS);
        word!("ثوان", SECONDS);
        word!("دقيقة", MINUTES);
        word!("دقائق", MINUTES);
        word!("ساعة", HOURS);
        word!("ساعات", HOURS);
        word!("أيام", DAYS);
        word!("يوم", DAYS);
        word!("أسابيع", WEEKS);
        word!("أسبوع", WEEKS);
        word!("أشهر", MONTHS);
        word!("شهر", MONTHS);
        word!("شهرين", MONTHS);
        word!("شهرًا", MONTHS);
        word!("سنة", YEARS);
        word!("سنوات", YEARS);
        word!("سنتين", YEARS);

        // Marathi
        word!("सेकंद", SECONDS);
        word!("मिनिटे", MINUTES);
        word!("मिनिट", MINUTES);
        word!("तास", HOURS);
        word!("दिवसांपूर्वी", DAYS);
        word!("दिवस", DAYS);
        word!("आठवड्यांपूर्वी", WEEKS);
        word!("आठवडा", WEEKS);
        word!("महिन्यांपूर्वी", MONTHS);
        word!("महिन्यापूर्वी", MONTHS);
        word!("महिना", MONTHS);
        word!("वर्षांपूर्वी", YEARS);
        word!("वर्षापूर्वी", YEARS);
        word!("वर्ष", YEARS);

        // Hindi
        word!("सेकंड", SECONDS);
        word!("मिनट", MINUTES);
        word!("घंटे", HOURS);
        word!("घंटा", HOURS);
        word!("दिन", DAYS);
        word!("सप्ताह", WEEKS);
        word!("हफ़्ते", WEEKS);
        word!("माह", MONTHS);
        word!("महीने", MONTHS);
        word!("महीना", MONTHS);
        word!("साल", YEARS);
        word!("वर्ष", YEARS);

        // Bengali
        word!("সেকেন্ড", SECONDS);
        word!("মিনিট", MINUTES);
        word!("ঘন্টা", HOURS);
        word!("ঘণ্টা", HOURS);
        word!("দিন", DAYS);
        word!("সপ্তাহ", WEEKS);
        word!("মাস", MONTHS);
        word!("বছর", YEARS);

        // Punjabi
        word!("ਸਕਿੰਟ", SECONDS);
        word!("ਮਿੰਟ", MINUTES);
        word!("ਘੰਟੇ", HOURS);
        word!("ਘੰਟਾ", HOURS);
        word!("ਦਿਨ", DAYS);
        word!("ਹਫ਼ਤੇ", WEEKS);
        word!("ਹਫ਼ਤਾ", WEEKS);
        word!("ਮਹੀਨੇ", MONTHS);
        word!("ਮਹੀਨਾ", MONTHS);
        word!("ਸਾਲ", YEARS);

        // Gujarati
        word!("સેકંડ", SECONDS);
        word!("કલાક", HOURS);
        word!("દિવસ", DAYS);
        word!("અઠવાડિયા", WEEKS);
        word!("અઠવાડિયું", WEEKS);
        word!("મહિના", MONTHS);
        word!("મહિનો", MONTHS);
        word!("વર્ષ", YEARS);

        // Tamil
        word!("வினாடிகளுக்கு", SECONDS);
        word!("வினாடி", SECONDS);
        word!("நிமிடங்களுக்கு", MINUTES);
        word!("நிமிடம்", MINUTES);
        word!("மணிநேரத்துக்கு", HOURS);
        word!("மணிநேரம்", HOURS);
        word!("நாட்களுக்கு", DAYS);
        word!("நாள்", DAYS);
        word!("வாரங்களுக்கு", WEEKS);
        word!("வாரம்", WEEKS);
        word!("மாதங்களுக்கு", MONTHS);
        word!("மாதத்துக்கு", MONTHS);
        word!("மாதம்", MONTHS);
        word!("ஆண்டுகளுக்கு", YEARS);
        word!("ஆண்டிற்கு", YEARS);
        word!("ஆண்டு", YEARS);

        // Telugu
        word!("సెకన్ల", SECONDS);
        word!("సెకను", SECONDS);
        word!("నిమిషాల", MINUTES);
        word!("నిమిషం", MINUTES);
        word!("గంటల", HOURS);
        word!("గంట", HOURS);
        word!("రోజుల", DAYS);
        word!("రోజు", DAYS);
        word!("వారాల", WEEKS);
        word!("వారం", WEEKS);
        word!("నెలల", MONTHS);
        word!("నెల", MONTHS);
        word!("సంవత్సరాల", YEARS);
        word!("సంవత్సరం", YEARS);

        // Thai
        word!("วินาที", SECONDS);
        word!("นาที", MINUTES);
        word!("ชั่วโมง", HOURS);
        word!("วัน", DAYS);
        word!("วันที่ผ่านมา", DAYS);
        word!("สัปดาห์", WEEKS);
        word!("สัปดาห์ที่ผ่านมา", WEEKS);
        word!("เดือน", MONTHS);
        word!("เดือนที่ผ่านมา", MONTHS);
        word!("ปี", YEARS);
        word!("ปีที่แล้ว", YEARS);

        // Georgian
        word!("წამის", SECONDS);
        word!("წამი", SECONDS);
        word!("წუთის", MINUTES);
        word!("წუთი", MINUTES);
        word!("საათის", HOURS);
        word!("საათი", HOURS);
        word!("დღის", DAYS);
        word!("დღე", DAYS);
        word!("კვირის", WEEKS);
        word!("კვირა", WEEKS);
        word!("თვის", MONTHS);
        word!("თვე", MONTHS);
        word!("წლის", YEARS);
        word!("წელი", YEARS);

        // Chinese (Simplified & Traditional) - CJK handled separately
        // Japanese - CJK handled separately
        // Korean - CJK handled separately

        root
    })
}

/// CJK keywords mapped to time multipliers (for contains-based matching)
static CJK_KEYWORDS: OnceLock<Vec<(&'static str, u64)>> = OnceLock::new();

fn get_cjk_keywords() -> &'static Vec<(&'static str, u64)> {
    CJK_KEYWORDS.get_or_init(|| {
        // Sorted by length descending for longest-match-first
        let mut keywords = vec![
            // Japanese (longer patterns first)
            ("週間", WEEKS),
            ("か月", MONTHS),
            ("ヶ月", MONTHS),
            ("時間", HOURS),
            // Chinese
            ("分钟", MINUTES),
            ("分鐘", MINUTES),
            ("小时", HOURS),
            ("小時", HOURS),
            ("个月", MONTHS),
            ("個月", MONTHS),
            // Korean
            ("시간", HOURS),
            ("개월", MONTHS),
            // Single chars (shorter, checked last)
            ("秒", SECONDS),
            ("分", MINUTES),
            ("天", DAYS),
            ("日", DAYS),
            ("周", WEEKS),
            ("週", WEEKS),
            ("年", YEARS),
            ("초", SECONDS),
            ("일", DAYS),
            ("주", WEEKS),
            ("년", YEARS),
        ];
        // Sort by length descending
        keywords.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
        keywords
    })
}

/// Check if text contains CJK characters
fn has_cjk(text: &str) -> bool {
    text.chars().any(|c| {
        matches!(c,
            '\u{4E00}'..='\u{9FFF}' |   // CJK Unified Ideographs
            '\u{3040}'..='\u{309F}' |   // Hiragana
            '\u{30A0}'..='\u{30FF}' |   // Katakana
            '\u{AC00}'..='\u{D7AF}'     // Hangul
        )
    })
}

/// Extract a number from text
fn extract_number(text: &str) -> u64 {
    let mut num_str = String::new();

    for c in text.chars() {
        if c.is_ascii_digit() {
            num_str.push(c);
        } else if !num_str.is_empty() {
            break;
        }
    }

    if num_str.is_empty() {
        for c in text.chars() {
            if c.is_ascii_digit() {
                num_str.push(c);
            }
        }
    }

    num_str.parse().unwrap_or(0)
}

/// Parse relative time text in any supported language into minutes for sorting.
/// Returns u64::MAX for unparseable strings (sorts to end).
pub fn parse_relative_time(text: Option<&str>) -> u64 {
    let text = match text {
        Some(t) if !t.is_empty() => t,
        _ => return u64::MAX,
    };

    let num = extract_number(text);
    let num = if num == 0 { 1 } else { num };

    // Handle CJK with contains-based matching (longest match wins)
    if has_cjk(text) {
        for (keyword, multiplier) in get_cjk_keywords() {
            if text.contains(keyword) {
                return num * multiplier;
            }
        }
    }

    // Split into words for trie lookup
    let words: Vec<&str> = text
        .split(|c: char| c.is_ascii_digit() || c.is_whitespace())
        .filter(|s| !s.is_empty())
        .collect();

    if words.is_empty() {
        return u64::MAX;
    }

    let trie = get_trie();

    // Try matching from each position in the word list
    for i in 0..words.len() {
        if let Some((multiplier, _)) = trie.lookup(&words[i..]) {
            return num * multiplier;
        }
    }

    u64::MAX
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_english() {
        assert_eq!(parse_relative_time(Some("5 minutes ago")), 5);
        assert_eq!(parse_relative_time(Some("2 hours ago")), 120);
        assert_eq!(parse_relative_time(Some("3 days ago")), 3 * 60 * 24);
        assert_eq!(parse_relative_time(Some("1 week ago")), 60 * 24 * 7);
        assert_eq!(parse_relative_time(Some("1 month ago")), 60 * 24 * 30);
        assert_eq!(parse_relative_time(Some("1 year ago")), 60 * 24 * 365);
    }

    #[test]
    fn test_german() {
        assert_eq!(parse_relative_time(Some("vor 9 Tagen")), 9 * 60 * 24);
        assert_eq!(parse_relative_time(Some("vor 3 Wochen")), 3 * 60 * 24 * 7);
        assert_eq!(parse_relative_time(Some("vor 1 Monat")), 60 * 24 * 30);
        assert_eq!(parse_relative_time(Some("vor 1 Jahr")), 60 * 24 * 365);
    }

    #[test]
    fn test_french() {
        assert_eq!(parse_relative_time(Some("il y a 9 jours")), 9 * 60 * 24);
        assert_eq!(
            parse_relative_time(Some("il y a 3 semaines")),
            3 * 60 * 24 * 7
        );
        assert_eq!(parse_relative_time(Some("il y a 1 mois")), 60 * 24 * 30);
        assert_eq!(parse_relative_time(Some("il y a 1 an")), 60 * 24 * 365);
    }

    #[test]
    fn test_azerbaijani() {
        assert_eq!(parse_relative_time(Some("9 gün öncə")), 9 * 60 * 24);
        assert_eq!(parse_relative_time(Some("1 il öncə")), 60 * 24 * 365);
    }

    #[test]
    fn test_japanese() {
        assert_eq!(parse_relative_time(Some("9 日前")), 9 * 60 * 24);
        assert_eq!(parse_relative_time(Some("3 週間前")), 3 * 60 * 24 * 7);
        assert_eq!(parse_relative_time(Some("1 か月前")), 60 * 24 * 30);
        assert_eq!(parse_relative_time(Some("1 年前")), 60 * 24 * 365);
    }

    #[test]
    fn test_chinese() {
        assert_eq!(parse_relative_time(Some("9天前")), 9 * 60 * 24);
        assert_eq!(parse_relative_time(Some("3周前")), 3 * 60 * 24 * 7);
        assert_eq!(parse_relative_time(Some("1个月前")), 60 * 24 * 30);
        assert_eq!(parse_relative_time(Some("1年前")), 60 * 24 * 365);
    }

    #[test]
    fn test_korean() {
        assert_eq!(parse_relative_time(Some("9일 전")), 9 * 60 * 24);
        assert_eq!(parse_relative_time(Some("3주 전")), 3 * 60 * 24 * 7);
        assert_eq!(parse_relative_time(Some("1개월 전")), 60 * 24 * 30);
        assert_eq!(parse_relative_time(Some("1년 전")), 60 * 24 * 365);
    }

    #[test]
    fn test_russian() {
        assert_eq!(parse_relative_time(Some("9 дней назад")), 9 * 60 * 24);
        assert_eq!(parse_relative_time(Some("3 недели назад")), 3 * 60 * 24 * 7);
        assert_eq!(parse_relative_time(Some("1 месяц назад")), 60 * 24 * 30);
        assert_eq!(parse_relative_time(Some("1 год назад")), 60 * 24 * 365);
    }

    #[test]
    fn test_invalid() {
        assert_eq!(parse_relative_time(None), u64::MAX);
        assert_eq!(parse_relative_time(Some("")), u64::MAX);
        assert_eq!(parse_relative_time(Some("invalid")), u64::MAX);
    }
    // ==========================================
    // All YouTube locale tests (auto-generated)
    // ==========================================

    #[test]
    fn test_youtube_afrikaans() {
        assert_eq!(parse_relative_time(Some("9 dae gelede")), 12960);
        assert_eq!(parse_relative_time(Some("3 weke gelede")), 30240);
        assert_eq!(parse_relative_time(Some("1 maand gelede")), 43200);
        assert_eq!(parse_relative_time(Some("1 jaar gelede")), 525600);
    }

    #[test]
    fn test_youtube_azerbaijani() {
        assert_eq!(parse_relative_time(Some("9 gün öncə")), 12960);
        assert_eq!(parse_relative_time(Some("3 həftə öncə")), 30240);
        assert_eq!(parse_relative_time(Some("1 ay öncə")), 43200);
        assert_eq!(parse_relative_time(Some("1 il öncə")), 525600);
    }

    #[test]
    fn test_youtube_indonesian() {
        assert_eq!(parse_relative_time(Some("9 hari yang lalu")), 12960);
        assert_eq!(parse_relative_time(Some("3 minggu yang lalu")), 30240);
        assert_eq!(parse_relative_time(Some("1 bulan yang lalu")), 43200);
        assert_eq!(parse_relative_time(Some("1 tahun yang lalu")), 525600);
    }

    #[test]
    fn test_youtube_malay() {
        assert_eq!(parse_relative_time(Some("9 hari lalu")), 12960);
        assert_eq!(parse_relative_time(Some("3 minggu lalu")), 30240);
        assert_eq!(parse_relative_time(Some("1 bulan lalu")), 43200);
        assert_eq!(parse_relative_time(Some("1 tahun lalu")), 525600);
    }

    #[test]
    fn test_youtube_bosnian() {
        assert_eq!(parse_relative_time(Some("prije 9 dana")), 12960);
        assert_eq!(parse_relative_time(Some("prije 3 sedmice")), 30240);
        assert_eq!(parse_relative_time(Some("prije 1 mjesec")), 43200);
        assert_eq!(parse_relative_time(Some("prije 1 godinu")), 525600);
    }

    #[test]
    fn test_youtube_catalan() {
        assert_eq!(parse_relative_time(Some("fa 9 dies")), 12960);
        assert_eq!(parse_relative_time(Some("fa 3 setmanes")), 30240);
        assert_eq!(parse_relative_time(Some("fa 1 mes")), 43200);
        assert_eq!(parse_relative_time(Some("fa 1 any")), 525600);
    }

    #[test]
    fn test_youtube_danish() {
        assert_eq!(parse_relative_time(Some("for 9 dage siden")), 12960);
        assert_eq!(parse_relative_time(Some("for 3 uger siden")), 30240);
        assert_eq!(parse_relative_time(Some("for 1 måned siden")), 43200);
        assert_eq!(parse_relative_time(Some("for 1 år siden")), 525600);
    }

    #[test]
    fn test_youtube_german() {
        assert_eq!(parse_relative_time(Some("vor 9 Tagen")), 12960);
        assert_eq!(parse_relative_time(Some("vor 3 Wochen")), 30240);
        assert_eq!(parse_relative_time(Some("vor 1 Monat")), 43200);
        assert_eq!(parse_relative_time(Some("vor 1 Jahr")), 525600);
    }

    #[test]
    fn test_youtube_estonian() {
        assert_eq!(parse_relative_time(Some("9 päeva eest")), 12960);
        assert_eq!(parse_relative_time(Some("3 nädala eest")), 30240);
        assert_eq!(parse_relative_time(Some("1 kuu eest")), 43200);
        assert_eq!(parse_relative_time(Some("1 aasta eest")), 525600);
    }

    #[test]
    fn test_youtube_english() {
        assert_eq!(parse_relative_time(Some("9 days ago")), 12960);
        assert_eq!(parse_relative_time(Some("3 weeks ago")), 30240);
        assert_eq!(parse_relative_time(Some("1 month ago")), 43200);
        assert_eq!(parse_relative_time(Some("1 year ago")), 525600);
    }

    #[test]
    fn test_youtube_spanish() {
        assert_eq!(parse_relative_time(Some("hace 9 días")), 12960);
        assert_eq!(parse_relative_time(Some("hace 3 semanas")), 30240);
        assert_eq!(parse_relative_time(Some("hace 1 mes")), 43200);
        assert_eq!(parse_relative_time(Some("hace 1 año")), 525600);
    }

    #[test]
    fn test_youtube_basque() {
        assert_eq!(parse_relative_time(Some("duela 9 egun")), 12960);
        assert_eq!(parse_relative_time(Some("duela 3 aste")), 30240);
        assert_eq!(parse_relative_time(Some("duela 1 hilabete")), 43200);
        assert_eq!(parse_relative_time(Some("duela 1 urte")), 525600);
    }

    #[test]
    fn test_youtube_french() {
        assert_eq!(parse_relative_time(Some("il y a 9 jours")), 12960);
        assert_eq!(parse_relative_time(Some("il y a 3 semaines")), 30240);
        assert_eq!(parse_relative_time(Some("il y a 1 mois")), 43200);
        assert_eq!(parse_relative_time(Some("il y a 1 an")), 525600);
    }

    #[test]
    fn test_youtube_croatian() {
        assert_eq!(parse_relative_time(Some("prije 9 dana")), 12960);
        assert_eq!(parse_relative_time(Some("prije 3 tjedna")), 30240);
        assert_eq!(parse_relative_time(Some("prije 1 mjesec")), 43200);
        assert_eq!(parse_relative_time(Some("prije 1 godinu")), 525600);
    }

    #[test]
    fn test_youtube_italian() {
        assert_eq!(parse_relative_time(Some("9 giorni fa")), 12960);
        assert_eq!(parse_relative_time(Some("3 settimane fa")), 30240);
        assert_eq!(parse_relative_time(Some("1 mese fa")), 43200);
        assert_eq!(parse_relative_time(Some("1 anno fa")), 525600);
    }

    #[test]
    fn test_youtube_swahili() {
        assert_eq!(parse_relative_time(Some("siku 9 zilizopita")), 12960);
        assert_eq!(parse_relative_time(Some("wiki 3 zilizopita")), 30240);
        assert_eq!(parse_relative_time(Some("mwezi 1 uliopita")), 43200);
        assert_eq!(parse_relative_time(Some("mwaka 1 uliopita")), 525600);
    }

    #[test]
    fn test_youtube_latvian() {
        assert_eq!(parse_relative_time(Some("pirms 9 dienām")), 12960);
        assert_eq!(parse_relative_time(Some("pirms 3 nedēļām")), 30240);
        assert_eq!(parse_relative_time(Some("pirms 1 mēneša")), 43200);
        assert_eq!(parse_relative_time(Some("pirms 1 gada")), 525600);
    }

    #[test]
    fn test_youtube_lithuanian() {
        assert_eq!(parse_relative_time(Some("prieš 9 dienas")), 12960);
        assert_eq!(parse_relative_time(Some("prieš 3 savaites")), 30240);
        assert_eq!(parse_relative_time(Some("prieš 1 mėnesį")), 43200);
        assert_eq!(parse_relative_time(Some("prieš 1 metus")), 525600);
    }

    #[test]
    fn test_youtube_hungarian() {
        assert_eq!(parse_relative_time(Some("9 nappal ezelőtt")), 12960);
        assert_eq!(parse_relative_time(Some("3 héttel ezelőtt")), 30240);
        assert_eq!(parse_relative_time(Some("1 hónappal ezelőtt")), 43200);
        assert_eq!(parse_relative_time(Some("1 évvel ezelőtt")), 525600);
    }

    #[test]
    fn test_youtube_dutch() {
        assert_eq!(parse_relative_time(Some("9 dagen geleden")), 12960);
        assert_eq!(parse_relative_time(Some("3 weken geleden")), 30240);
        assert_eq!(parse_relative_time(Some("1 maand geleden")), 43200);
        assert_eq!(parse_relative_time(Some("1 jaar geleden")), 525600);
    }

    #[test]
    fn test_youtube_norwegian() {
        assert_eq!(parse_relative_time(Some("for 9 døgn siden")), 12960);
        assert_eq!(parse_relative_time(Some("for 3 uker siden")), 30240);
        assert_eq!(parse_relative_time(Some("for 1 måned siden")), 43200);
        assert_eq!(parse_relative_time(Some("for 1 år siden")), 525600);
    }

    #[test]
    fn test_youtube_polish() {
        assert_eq!(parse_relative_time(Some("9 dni temu")), 12960);
        assert_eq!(parse_relative_time(Some("3 tygodnie temu")), 30240);
        assert_eq!(parse_relative_time(Some("1 miesiąc temu")), 43200);
        assert_eq!(parse_relative_time(Some("1 rok temu")), 525600);
    }

    #[test]
    fn test_youtube_portuguese() {
        assert_eq!(parse_relative_time(Some("há 9 dias")), 12960);
        assert_eq!(parse_relative_time(Some("há 3 semanas")), 30240);
        assert_eq!(parse_relative_time(Some("há 1 mês")), 43200);
        assert_eq!(parse_relative_time(Some("há 1 ano")), 525600);
    }

    #[test]
    fn test_youtube_romanian() {
        assert_eq!(parse_relative_time(Some("acum 9 zile")), 12960);
        assert_eq!(parse_relative_time(Some("acum 3 săptămâni")), 30240);
        assert_eq!(parse_relative_time(Some("acum 1 lună")), 43200);
        assert_eq!(parse_relative_time(Some("acum 1 an")), 525600);
    }

    #[test]
    fn test_youtube_slovak() {
        assert_eq!(parse_relative_time(Some("pred 9 dňami")), 12960);
        assert_eq!(parse_relative_time(Some("pred 3 týždňami")), 30240);
        assert_eq!(parse_relative_time(Some("pred 1 mesiacom")), 43200);
        assert_eq!(parse_relative_time(Some("pred 1 rokom")), 525600);
    }

    #[test]
    fn test_youtube_slovene() {
        assert_eq!(parse_relative_time(Some("pred 4 dnevi")), 5760);
        assert_eq!(parse_relative_time(Some("pred 3 tedni")), 30240);
        assert_eq!(parse_relative_time(Some("pred 1 mesecem")), 43200);
        assert_eq!(parse_relative_time(Some("pred 1 letom")), 525600);
    }

    #[test]
    fn test_youtube_finnish() {
        assert_eq!(parse_relative_time(Some("9 päivää sitten")), 12960);
        assert_eq!(parse_relative_time(Some("3 viikkoa sitten")), 30240);
        assert_eq!(parse_relative_time(Some("1 kuukausi sitten")), 43200);
        assert_eq!(parse_relative_time(Some("1 vuosi sitten")), 525600);
    }

    #[test]
    fn test_youtube_swedish() {
        assert_eq!(parse_relative_time(Some("för 9 dagar sedan")), 12960);
        assert_eq!(parse_relative_time(Some("för 3 veckor sedan")), 30240);
        assert_eq!(parse_relative_time(Some("för 1 månad sedan")), 43200);
        assert_eq!(parse_relative_time(Some("för 1 år sedan")), 525600);
    }

    #[test]
    fn test_youtube_tagalog() {
        assert_eq!(parse_relative_time(Some("9 araw ang nakalipas")), 12960);
        assert_eq!(parse_relative_time(Some("3 linggo ang nakalipas")), 30240);
        assert_eq!(parse_relative_time(Some("1 buwan ang nakalipas")), 43200);
        assert_eq!(parse_relative_time(Some("1 taon ang nakalipas")), 525600);
    }

    #[test]
    fn test_youtube_vietnamese() {
        assert_eq!(parse_relative_time(Some("9 ngày trước")), 12960);
        assert_eq!(parse_relative_time(Some("3 tuần trước")), 30240);
        assert_eq!(parse_relative_time(Some("1 tháng trước")), 43200);
        assert_eq!(parse_relative_time(Some("1 năm trước")), 525600);
    }

    #[test]
    fn test_youtube_turkish() {
        assert_eq!(parse_relative_time(Some("9 gün önce")), 12960);
        assert_eq!(parse_relative_time(Some("3 hafta önce")), 30240);
        assert_eq!(parse_relative_time(Some("1 ay önce")), 43200);
        assert_eq!(parse_relative_time(Some("1 yıl önce")), 525600);
    }

    #[test]
    fn test_youtube_icelandic() {
        assert_eq!(parse_relative_time(Some("fyrir 9 dögum")), 12960);
        assert_eq!(parse_relative_time(Some("fyrir 3 vikum")), 30240);
        assert_eq!(parse_relative_time(Some("fyrir 1 mánuði")), 43200);
        assert_eq!(parse_relative_time(Some("fyrir 1 ári")), 525600);
    }

    #[test]
    fn test_youtube_czech() {
        assert_eq!(parse_relative_time(Some("před 9 dny")), 12960);
        assert_eq!(parse_relative_time(Some("před 3 týdny")), 30240);
        assert_eq!(parse_relative_time(Some("před 1 měsícem")), 43200);
        assert_eq!(parse_relative_time(Some("před 1 rokem")), 525600);
    }

    #[test]
    fn test_youtube_greek() {
        assert_eq!(parse_relative_time(Some("πριν από 9 ημέρες")), 12960);
        assert_eq!(parse_relative_time(Some("πριν από 3 εβδομάδες")), 30240);
        assert_eq!(parse_relative_time(Some("πριν από 1 μήνα")), 43200);
        assert_eq!(parse_relative_time(Some("πριν από 1 έτος")), 525600);
    }

    #[test]
    fn test_youtube_belarusian() {
        assert_eq!(parse_relative_time(Some("9 дзён таму")), 12960);
        assert_eq!(parse_relative_time(Some("3 тыдні таму")), 30240);
        assert_eq!(parse_relative_time(Some("1 месяц таму")), 43200);
        assert_eq!(parse_relative_time(Some("1 год таму")), 525600);
    }

    #[test]
    fn test_youtube_bulgarian() {
        assert_eq!(parse_relative_time(Some("преди 9 дни")), 12960);
        assert_eq!(parse_relative_time(Some("преди 3 седмици")), 30240);
        assert_eq!(parse_relative_time(Some("преди 1 месец")), 43200);
        assert_eq!(parse_relative_time(Some("преди 1 година")), 525600);
    }

    #[test]
    fn test_youtube_macedonian() {
        assert_eq!(parse_relative_time(Some("пред 9 дена")), 12960);
        assert_eq!(parse_relative_time(Some("пред 3 седмици")), 30240);
        assert_eq!(parse_relative_time(Some("пред 1 месец")), 43200);
        assert_eq!(parse_relative_time(Some("пред 1 година")), 525600);
    }

    #[test]
    fn test_youtube_mongolian() {
        assert_eq!(parse_relative_time(Some("9 өдрийн өмнө")), 12960);
        assert_eq!(parse_relative_time(Some("3 долоо хоногийн өмнө")), 30240);
        assert_eq!(parse_relative_time(Some("1 сарын өмнө")), 43200);
        assert_eq!(parse_relative_time(Some("1 жилийн өмнө")), 525600);
    }

    #[test]
    fn test_youtube_russian() {
        assert_eq!(parse_relative_time(Some("9 дней назад")), 12960);
        assert_eq!(parse_relative_time(Some("3 недели назад")), 30240);
        assert_eq!(parse_relative_time(Some("1 месяц назад")), 43200);
        assert_eq!(parse_relative_time(Some("1 год назад")), 525600);
    }

    #[test]
    fn test_youtube_serbian() {
        assert_eq!(parse_relative_time(Some("пре 9 дана")), 12960);
        assert_eq!(parse_relative_time(Some("пре 3 недеље")), 30240);
        assert_eq!(parse_relative_time(Some("пре 1 месеца")), 43200);
        assert_eq!(parse_relative_time(Some("пре 1 године")), 525600);
    }

    #[test]
    fn test_youtube_ukrainian() {
        assert_eq!(parse_relative_time(Some("9 днів тому")), 12960);
        assert_eq!(parse_relative_time(Some("3 тижні тому")), 30240);
        assert_eq!(parse_relative_time(Some("1 місяць тому")), 43200);
        assert_eq!(parse_relative_time(Some("1 рік тому")), 525600);
    }

    #[test]
    fn test_youtube_kazakh() {
        assert_eq!(parse_relative_time(Some("9 күн бұрын")), 12960);
        assert_eq!(parse_relative_time(Some("3 апта бұрын")), 30240);
        assert_eq!(parse_relative_time(Some("1 ай бұрын")), 43200);
        assert_eq!(parse_relative_time(Some("1 жыл бұрын")), 525600);
    }

    #[test]
    fn test_youtube_armenian() {
        assert_eq!(parse_relative_time(Some("9 օր առաջ")), 12960);
        assert_eq!(parse_relative_time(Some("3 շաբաթ առաջ")), 30240);
        assert_eq!(parse_relative_time(Some("1 ամիս առաջ")), 43200);
        assert_eq!(parse_relative_time(Some("1 տարի առաջ")), 525600);
    }

    #[test]
    fn test_youtube_hebrew() {
        assert_eq!(parse_relative_time(Some("לפני 9 ימים")), 12960);
        assert_eq!(parse_relative_time(Some("לפני 3 שבועות")), 30240);
        assert_eq!(parse_relative_time(Some("לפני חודש (1)")), 43200);
        assert_eq!(parse_relative_time(Some("לפני שנה")), 525600);
    }

    #[test]
    fn test_youtube_urdu() {
        assert_eq!(parse_relative_time(Some("9 دنوں پہلے")), 12960);
        assert_eq!(parse_relative_time(Some("3 ہفتے پہلے")), 30240);
        assert_eq!(parse_relative_time(Some("1 مہینہ پہلے")), 43200);
        assert_eq!(parse_relative_time(Some("1 سال پہلے")), 525600);
    }

    #[test]
    fn test_youtube_arabic() {
        assert_eq!(parse_relative_time(Some("قبل 9 أيام")), 12960);
        assert_eq!(parse_relative_time(Some("قبل 3 أسابيع")), 30240);
        assert_eq!(parse_relative_time(Some("قبل شهر واحد")), 43200);
        assert_eq!(parse_relative_time(Some("قبل سنة واحدة")), 525600);
    }

    #[test]
    fn test_youtube_marathi() {
        assert_eq!(parse_relative_time(Some("9 दिवसांपूर्वी")), 12960);
        assert_eq!(parse_relative_time(Some("3 आठवड्यांपूर्वी")), 30240);
        assert_eq!(parse_relative_time(Some("1 महिन्यापूर्वी")), 43200);
        assert_eq!(parse_relative_time(Some("1 वर्षापूर्वी")), 525600);
    }

    #[test]
    fn test_youtube_hindi() {
        assert_eq!(parse_relative_time(Some("9 दिन पहले")), 12960);
        assert_eq!(parse_relative_time(Some("3 सप्ताह पहले")), 30240);
        assert_eq!(parse_relative_time(Some("1 माह पहले")), 43200);
        assert_eq!(parse_relative_time(Some("1 वर्ष पहले")), 525600);
    }

    #[test]
    fn test_youtube_bengali() {
        assert_eq!(parse_relative_time(Some("9 দিন আগে")), 12960);
        assert_eq!(parse_relative_time(Some("3 সপ্তাহ আগে")), 30240);
        assert_eq!(parse_relative_time(Some("1 মাস আগে")), 43200);
        assert_eq!(parse_relative_time(Some("1 বছর পূর্বে")), 525600);
    }

    #[test]
    fn test_youtube_punjabi() {
        assert_eq!(parse_relative_time(Some("9 ਦਿਨ ਪਹਿਲਾਂ")), 12960);
        assert_eq!(parse_relative_time(Some("3 ਹਫ਼ਤੇ ਪਹਿਲਾਂ")), 30240);
        assert_eq!(parse_relative_time(Some("1 ਮਹੀਨਾ ਪਹਿਲਾਂ")), 43200);
        assert_eq!(parse_relative_time(Some("1 ਸਾਲ ਪਹਿਲਾਂ")), 525600);
    }

    #[test]
    fn test_youtube_gujarati() {
        assert_eq!(parse_relative_time(Some("9 દિવસ પહેલાં")), 12960);
        assert_eq!(parse_relative_time(Some("3 અઠવાડિયા પહેલાં")), 30240);
        assert_eq!(parse_relative_time(Some("1 મહિના પહેલાં")), 43200);
        assert_eq!(parse_relative_time(Some("1 વર્ષ પહેલાં")), 525600);
    }

    #[test]
    fn test_youtube_tamil() {
        assert_eq!(parse_relative_time(Some("9 நாட்களுக்கு முன்")), 12960);
        assert_eq!(parse_relative_time(Some("3 வாரங்களுக்கு முன்")), 30240);
        assert_eq!(parse_relative_time(Some("1 மாதத்துக்கு முன்")), 43200);
        assert_eq!(parse_relative_time(Some("1 ஆண்டிற்கு முன்")), 525600);
    }

    #[test]
    fn test_youtube_telugu() {
        assert_eq!(parse_relative_time(Some("9 రోజుల క్రితం")), 12960);
        assert_eq!(parse_relative_time(Some("3 వారాల క్రితం")), 30240);
        assert_eq!(parse_relative_time(Some("1 నెల క్రితం")), 43200);
        assert_eq!(parse_relative_time(Some("1 సంవత్సరం క్రితం")), 525600);
    }

    #[test]
    fn test_youtube_thai() {
        assert_eq!(parse_relative_time(Some("9 วันที่ผ่านมา")), 12960);
        assert_eq!(parse_relative_time(Some("3 สัปดาห์ที่ผ่านมา")), 30240);
        assert_eq!(parse_relative_time(Some("1 เดือนที่ผ่านมา")), 43200);
        assert_eq!(parse_relative_time(Some("1 ปีที่แล้ว")), 525600);
    }

    #[test]
    fn test_youtube_georgian() {
        assert_eq!(parse_relative_time(Some("9 დღის წინ")), 12960);
        assert_eq!(parse_relative_time(Some("3 კვირის წინ")), 30240);
        assert_eq!(parse_relative_time(Some("1 თვის წინ")), 43200);
        assert_eq!(parse_relative_time(Some("1 წლის წინ")), 525600);
    }

    #[test]
    fn test_youtube_chinese() {
        assert_eq!(parse_relative_time(Some("9天前")), 12960);
        assert_eq!(parse_relative_time(Some("3周前")), 30240);
        assert_eq!(parse_relative_time(Some("1个月前")), 43200);
        assert_eq!(parse_relative_time(Some("1年前")), 525600);
    }

    #[test]
    fn test_youtube_japanese() {
        assert_eq!(parse_relative_time(Some("9 日前")), 12960);
        assert_eq!(parse_relative_time(Some("3 週間前")), 30240);
        assert_eq!(parse_relative_time(Some("1 か月前")), 43200);
        assert_eq!(parse_relative_time(Some("1 年前")), 525600);
    }

    #[test]
    fn test_youtube_korean() {
        assert_eq!(parse_relative_time(Some("9일 전")), 12960);
        assert_eq!(parse_relative_time(Some("3주 전")), 30240);
        assert_eq!(parse_relative_time(Some("1개월 전")), 43200);
        assert_eq!(parse_relative_time(Some("1년 전")), 525600);
    }
}
