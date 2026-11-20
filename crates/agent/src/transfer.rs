// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Bageo-Commercial
// SPDX-FileCopyrightText: h4lo-void <https://github.com/h4lo-void/bageo-call>

//! Who a call may be forwarded for (`whitelist_mode` direct or emergency), the
//! owner's urgent alert, and phone numbers in logs.
//!
//! direct: the phone declines the call and the carrier's busy forwarding rings
//! the owner. emergency: the AI answers; when the caller has an urgent matter
//! the owner is alerted and the AI tells the caller the owner will call back.

use crate::locale::Lang;
use crate::settings_store::RuntimeSettings;

/// Who a call that may be forwarded to the owner comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferWho {
    /// A Whitelist contact (setting `transfer_whitelist`).
    Whitelist,
    /// Any other contact off the Blacklist and the stall list (`transfer_contacts`).
    Contact,
    /// A number not in the contacts, hidden numbers too (`transfer_others`).
    Stranger,
}

/// A call that may be forwarded: who it is from, and the contact's name
/// (empty for a number not in the contacts).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transfer {
    pub who: TransferWho,
    pub name: String,
}

/// Whether a caller may be forwarded (`whitelist_mode` direct or emergency).
/// `kind`: the contact list the number is on (None: not in the contacts);
/// `blocked`: the number is turned away (Blacklist, also by a prefix, or a
/// blocked number of an older version). Blacklist and stall-list contacts never
/// are; a Whitelist contact is even when a Blacklist prefix covers its number.
pub fn transfer_who(rt: &RuntimeSettings, kind: Option<&str>, blocked: bool) -> Option<TransferWho> {
    if !matches!(rt.whitelist_mode.as_str(), "direct" | "emergency") {
        return None;
    }
    match kind {
        Some("whitelist") => rt.transfer_whitelist.then_some(TransferWho::Whitelist),
        Some("" | "contact") => (rt.transfer_contacts && !blocked).then_some(TransferWho::Contact),
        Some(_) => None,
        None => (rt.transfer_others && !blocked).then_some(TransferWho::Stranger),
    }
}

/// [`transfer_who`] for the number `caller` on a call that came in on the SIM
/// of country `sim`. The one place that decides whether a call is forwarded
/// (direct) or gets the urgent part and the alert (emergency). Our
/// own calls (call-backs, errands) are left out by the callers.
pub fn transfer_target(rt: &RuntimeSettings, caller: &str, sim: Option<&str>) -> Option<Transfer> {
    if !matches!(rt.whitelist_mode.as_str(), "direct" | "emergency") {
        return None;
    }
    let contact = crate::contacts::is_whitelist(caller, sim).or_else(|| crate::contacts::lookup(caller, sim));
    let blocked = crate::settings_store::reject_number_hit_on(rt.pending_rejects(), caller, sim);
    let who = transfer_who(rt, contact.as_ref().map(|c| c.kind.as_str()), blocked)?;
    Some(Transfer {
        who,
        name: contact.map(|c| c.name).unwrap_or_default(),
    })
}

/// A call judged sales / scam (stalling started, or a judgement that stands)
/// gets no urgent alert, whoever the caller is.
pub fn urgent_blocked_by_scam(stall_engaged: bool, judged: Option<&crate::calls::ScamJudgement>) -> bool {
    stall_engaged || judged.is_some_and(crate::calls::ScamJudgement::stands)
}

/// Title of the owner's urgent alert (same as the urgent-matter alert).
pub fn alert_title(lang: Lang) -> &'static str {
    match lang {
        Lang::ZhTw => "來電急事通知",
        Lang::ZhCn => "来电急事通知",
        Lang::Ja => "急ぎの着信のお知らせ",
        Lang::Ko => "급한 전화 알림",
        Lang::Es => "Aviso: llamada urgente",
        Lang::Fr => "Alerte : appel urgent",
        Lang::De => "Dringender Anruf",
        Lang::Pt => "Aviso: ligação urgente",
        Lang::En => "Urgent Call Alert",
    }
}

/// Last four digits only, for logs.
pub fn masked(number: &str) -> String {
    let digits: Vec<char> = number.chars().filter(|c| c.is_ascii_digit()).collect();
    let tail: String = digits[digits.len().saturating_sub(4)..].iter().collect();
    format!("***{tail}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logs_show_only_the_last_four_digits() {
        assert_eq!(masked("+1 (555) 000-1111"), "***1111");
        assert_eq!(masked("12"), "***12");
        for n in ["+1 (555) 000-1111", "0912-345-678", "+886 912 345 678", "tel:+15550001111;ext=22", "", "abc"] {
            let m = masked(n);
            assert!(m.starts_with("***"), "{m}");
            assert!(m.chars().filter(|c| c.is_ascii_digit()).count() <= 4, "{m}");
        }
        assert_eq!(masked(""), "***");
    }

    #[test]
    fn the_alert_title_exists_in_every_language() {
        for l in crate::locale::ALL {
            assert!(!alert_title(l).trim().is_empty(), "{l:?}");
        }
    }

    // ---- Nine-language table: urgent turns and hold-on lines ----

    /// (language, three real emergencies, three mentions that are not urgent)
    const URGENT_TABLE: &[(&str, [&str; 3], [&str; 3])] = &[
        ("zh-TW", ["我是他姐姐，我爸爸剛被送急診，請馬上找他。", "有急事，麻煩你叫他現在馬上接電話。", "他出車禍了，很急，我要馬上找本人。"],
                  ["明天想約你吃飯，有空再回我。", "最近天氣變涼，記得多穿點。", "我只是打來問候一下，不用特地找他。"]),
        ("zh-CN", ["我是他表哥，我妈妈刚被送去急诊，请马上找他。", "有紧急的事，请让他马上接电话。", "他出车祸了，很急，我要马上找本人。"],
                  ["明天想约你吃饭，有空再回我。", "最近天气变凉，记得多穿点。", "我只是打来问候一下，不用特地找他。"]),
        ("en", ["This is his sister, our dad was just taken to the hospital, I need to reach him right now.", "It's an emergency, please get him on the phone.", "There's been an accident and I need him immediately."],
               ["Want to grab lunch tomorrow?", "Just wanted to say hi, no need to find him.", "Tell him the book he lent me was great."]),
        ("ja", ["兄の友人です。母が事故に遭いました、至急本人につないでください。", "急ぎの用事なんです、すぐ繋いでください。", "父が病院に運ばれました、本人と話したいです。"],
               ["明日ご飯でもどうですか。", "ご無沙汰しています、挨拶だけです。", "お借りした本、とても面白かったと伝えてください。"]),
        ("ko", ["그의 누나인데요, 아버지가 응급실에 실려 갔어요. 지금 바로 연결해 주세요.", "긴급한 일이에요, 본인과 통화해야 해요.", "급한 일이 생겼어요, 빨리 바꿔 주세요."],
               ["내일 밥 한번 먹어요.", "그냥 인사하려고 전화했어요.", "빌려준 책 재미있었다고 전해 주세요."]),
        ("es", ["Soy su hermana, mi padre acaba de ingresar al hospital, necesito hablar con él.", "Es una emergencia, por favor comuníquelo ahora mismo.", "Hubo un accidente y es urgente que hable con él."],
               ["¿Quieres comer mañana?", "Solo llamo para saludar.", "Dile que el libro que me prestó me encantó."]),
        ("fr", ["C'est sa sœur, notre père vient d'être emmené à l'hôpital, je dois lui parler.", "C'est urgent, passez-le-moi tout de suite.", "Il y a eu une urgence, j'ai besoin de lui immédiatement."],
               ["On déjeune demain ?", "J'appelais juste pour dire bonjour.", "Dis-lui que son livre m'a beaucoup plu."]),
        ("de", ["Ich bin seine Schwester, unser Vater ist im Krankenhaus, ich muss ihn sofort sprechen.", "Es ist ein Notfall, bitte verbinden Sie mich.", "Es gab einen Unfall, es ist dringend."],
               ["Hast du morgen Zeit zum Mittagessen?", "Ich rufe nur an, um Hallo zu sagen.", "Sag ihm, das Buch war richtig gut."]),
        ("pt", ["Sou irmã dele, meu pai foi levado para o hospital, preciso falar com ele agora mesmo.", "É uma emergência, por favor chame-o.", "Houve um acidente, é urgente."],
               ["Quer almoçar amanhã?", "Só liguei para dizer oi.", "Diga a ele que o livro emprestado era ótimo."]),
    ];

    /// Mentions of hospital / accident / "right now" with nothing urgent behind them.
    /// `is_urgent_whitelist_turn` matches the bare word, so these come out true.
    const MENTION_ONLY: &[(&str, &str)] = &[
        ("zh-TW", "我上週去醫院拿藥，沒事了。"),
        ("zh-TW", "好意外你還記得我，哈哈。"),
        ("zh-CN", "我上周去医院拿药，没事了。"),
        ("zh-CN", "好意外你还记得我，哈哈。"),
        ("en", "I went to the hospital last week to pick up my meds, nothing serious."),
        ("en", "Right now I'm just calling to say hi."),
        ("ja", "先週、病院で薬をもらいましたが、大丈夫です。"),
        ("ja", "最近は仕事が大変ですね。"),
        ("ko", "지난주에 병원에서 약을 받았는데 괜찮아요."),
        ("ko", "시간이 빨리 가네요, 벌써 가을이에요."),
        ("es", "La semana pasada fui al hospital por mis medicinas, nada grave."),
        ("fr", "La semaine dernière je suis allé à l'hôpital chercher mes médicaments, rien de grave."),
        ("de", "Letzte Woche war ich im Krankenhaus wegen meiner Tabletten, nichts Schlimmes."),
        ("pt", "Semana passada fui ao hospital buscar remédio, nada grave."),
    ];

    #[test]
    fn urgent_turns_are_found_in_every_language() {
        for (lang, urgent, _) in URGENT_TABLE {
            for s in urgent {
                assert!(crate::gemini_live::is_urgent_whitelist_turn(s), "{lang}: {s}");
            }
        }
    }

    #[test]
    fn friendly_turns_without_an_emergency_are_not_urgent_in_every_language() {
        for (lang, _, calm) in URGENT_TABLE {
            for s in calm {
                assert!(!crate::gemini_live::is_urgent_whitelist_turn(s), "{lang}: {s}");
            }
        }
    }

    #[test]
    fn known_gap_mentioning_a_hospital_without_urgency_is_not_urgent() {
        for (lang, s) in MENTION_ONLY {
            assert!(!crate::gemini_live::is_urgent_whitelist_turn(s), "{lang}: {s}");
        }
    }
}
