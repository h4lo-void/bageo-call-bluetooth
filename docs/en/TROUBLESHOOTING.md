# Troubleshooting & FAQ

Your AI phone assistant rang out, hung up in silence, or the phone never showed
up? Find the message or symptom you see in the tables below: each row says why
it happens and how to fix it. The FAQ at the end answers the common questions
about iPhones, cost and keeping your real phone number usable.

## Look here first

1. Open the **System** page of the web console. Its checklist names the piece
   that is missing.
2. Watch Bageo's terminal window on the computer: it shows what it is doing on
   every call. (The Windows and macOS installers run Bageo in the background
   with **Bageo Status** and no terminal window; run `start.cmd -Console` on
   Windows or `./start.sh --console` on Linux to see it.) For more detail, set
   `RUST_LOG=agent=debug` in `.env` and restart. The log shows only how many
   characters were said on a call; to see the words, set `LOG_CALL_TEXT=1` too,
   and turn it off when done.
3. On the Bluetooth computer, run `journalctl -u bageo-bluetooth -f`. "Phone
   connected" means the phone is linked, and each call shows its steps.

## Bageo does not start

| You see | Why | Fix |
|---|---|---|
| `cannot listen on 0.0.0.0:8787` or `127.0.0.1:8787 … is another copy running?` | Another copy of Bageo, or another program, already uses the port. | Close it, or set `AGENT_PORT=8790` in `.env`. |
| `The agent program was not found.` | You cloned the source and Rust is not installed, so there is nothing to run. | Install [Rust](https://rustup.rs) and run the start script again, or install Bageo from the Windows installer or macOS `.dmg` on GitHub Releases. |
| Windows: "running scripts is disabled on this system" | PowerShell blocks `.ps1` files by default. | Click the **Bageo** shortcut, or start `start.cmd` in the install folder instead of `start.ps1`; it launches PowerShell with the right option. |
| macOS: "cannot be opened because the developer cannot be verified" | The app is not notarised by Apple. | In Finder, right-click Bageo → *Open*, or press *Open Anyway* in *System Settings → Privacy & Security*. |
| The console asks for a password | A console password or `CALLS_UI_TOKEN` is set. | Type the password (the browser remembers it), or open `http://…:8787/?token=<your password>`. |

## The phone does not show up

*System → Phones* shows how far the connection got. The Bluetooth program's own
log is on the Bluetooth computer: `journalctl -u bageo-bluetooth -f`.

| You see | Why | Fix |
|---|---|---|
| No "Phone connected" in the log | The phone is not paired, its Bluetooth is off, or it is out of range (about 10 m). | Switch Bluetooth on, move the phone closer, or pair again ([Phone setup](PHONE.md#pair-your-phone)). |
| Android does not list "Bageo HF" | Pairing stays open for 10 minutes after the service starts. | Run `sudo systemctl restart bageo-bluetooth`, then pair within 10 minutes. |
| The iPhone does not list the computer | iPhone doesn't list computers. | Pair from the Bluetooth computer: `sudo bageo-bluetooth pair`. |
| The phone is paired, but *System → Phones* is empty | Bageo runs on another computer or in Docker and the Bluetooth computer is not linked to it, or *Phones can also connect over Wi-Fi* is off. | Get a new pairing code and run `sudo bageo-bluetooth link <address> <code>`, then restart the service ([Phone setup](PHONE.md#connect-it-to-bageo)). |
| Calls break up or the phone keeps dropping | On a desktop Linux, PipeWire or PulseAudio also take Bluetooth hands-free. | Use a separate USB Bluetooth adapter, or turn off their hands-free support. |

## Calls are not answered, or sound wrong

| Symptom | Why | Fix |
|---|---|---|
| The phone rings on and on | The phone is in [normal phone mode](PHONE.md#normal-phone-mode) (*System → Phones* says *Normal phone*), the Bluetooth computer is off or out of range, or it has no connection to Bageo. Then nothing picks up, and the call rings until your carrier counts it as missed or sends it to voicemail. | Check *System → Phones* and the Bluetooth log; switch the phone back to AI answering. |
| Picked up, then hung up at once, nothing said | The AI could not start: no Gemini key, a wrong key, an unknown model name, no internet, or *AI answers calls* is off. The call's summary says "The AI did not connect and said nothing"; the terminal shows Gemini's error. | Fix the key or model, check the internet connection, switch *AI answers calls* on. |
| The caller hears nothing, yet the transcript has AI lines | The Bluetooth link does not carry sound back into the call. | On an iPhone keep *Call Audio Routing* on "Automatic"; pair again and read the Bluetooth log ([Phone settings](PHONE.md#phone-settings)). |
| The AI never reacts; the transcript has no caller lines | The phone sends no call sound over Bluetooth. | Same checks as above. |
| Calls from LINE, WhatsApp or FaceTime ring, but Bageo does not take them | Phones don't tell Bluetooth devices that these calls are ringing. | Turn on "Auto-Answer Calls" at 5 seconds ([Phone settings](PHONE.md#phone-settings)). |
| The AI answers itself or talks over the caller | The phone's recording contains the AI's own voice, and the per-call echo check did not catch it. | Switch off *Let the caller interrupt*. |
| The AI hangs up before the caller is done | Its prompt lets it say goodbye too early. | See the ending rule in [Writing the prompt](PROMPTS.md#writing-or-editing-it-yourself). |
| The call never ends | The caller stays on the line without a goodbye Bageo recognises. | It ends at *Longest normal call*; shorten that if it happens often. |
| A silent call ends after about half a minute | Intended: a pocket dial or a robot that only listens should not hold the line. | Nothing to fix; the timings are in the table below. |
| On a call from abroad the AI starts late, or talks over a recorded warning | Carriers in Taiwan play a fraud warning on calls from abroad. When a Taiwanese SIM takes the call, the AI waits until the line goes quiet (7.5 seconds at most) and then says its opening line once. | There is no setting for the wait. |
| A second caller says your line was busy | One call at a time, by design. A call that rings during a call is declined at once. | It is listed in the calls, and you get a "Missed while on another call" notification. |
| The first words come late | The AI only connects after pick-up. | Switch on *Get the AI ready while the phone rings* and keep *Ring before answering* at 3 s or more. |
| It answers in the wrong language | *Language* sets the opening line, and *Main language for each SIM* the opening on another country's SIM. | Check both, and make sure your prompt does not tie the AI to one language (for example "always answer in English"). |

Opened in the right language, then switched? That is intended: when the caller
clearly speaks another language, the AI follows. A short "hello?" alone does
not switch it. This is on by default; turn it off with *Reply in the caller's
language* under *Settings → How it answers*.

When the line goes quiet, Bageo follows fixed timings (no setting):

| Situation | What happens |
|---|---|
| The caller never says a word | The AI wraps up 25 seconds after its first words (about 27 seconds after pick-up) and hangs up. |
| The line goes quiet mid-call | After 15 seconds the AI asks "Are you still there?" (twice at most) and ends the call 25 seconds later. |
| The caller asked the AI to hold on | It asks after 45 seconds and ends after 90. |
| A recorded voice menu answers | The AI does not reply to it; Bageo hangs up after 4 seconds of quiet line. |

## Sales and scam calls

When a caller pitches a loan or claims to be "the bank's risk desk", Bageo can
keep them busy. The AI turns into a slow, curious listener who keeps asking how
it all works (*Keep them talking*).

If that catches the wrong caller, or a blocked number still gets through, see
the table:

| Symptom | Why | Fix |
|---|---|---|
| A prefix such as `080*` blocks nothing | A `*` pattern needs at least three digits before the star, not counting leading zeros (`0800*`), so one short line cannot block a whole region or every caller. | Lengthen the prefix. Write numbers the way the call list shows them; `+…` forms match only when your country code is known. |
| A real call was treated as sales or scam | The check starts from keywords, which honest callers sometimes use too, and a low-cost model takes a second look. The terminal names the words: `stall engaged … hits=[…]`. | On the call's page press *It was not* under *Sales / scam check*: the number is treated like a saved contact from then on. You can also switch off *Keep them talking*. |
| A friend or family member was kept talking | They are not saved as a contact. | Save them under *Contacts*. |

Before it starts keeping a contact talking, Bageo takes a second look: a
contact talking about money, transfers or bills is left alone. Requests like a
verification code or ATM steps still get kept talking (caller ID can be faked).

## Appointments

| You see | Why | Fix |
|---|---|---|
| No call-back option for a caller | Toll-free numbers, short codes, hidden numbers and numbers from abroad get no call-back. | *Default reply* then tells nobody ([Settings](SETTINGS.md#summary--calendar)). |
| The AI call-back said nothing | It waits for the callee to pick up and speak. Nobody within 70 seconds counts as `no_answer`. | *To do* shows how far it got. |
| A Telegram card still has buttons after you decided on the web | The buttons are removed a few seconds after the decision. | Nothing to do; pressing an old card only answers "This appointment has already been handled". |

## AI errands

An AI errand is a call the AI places for you from the *Errands* page, for
example "book a table for four on Friday at 7 pm, outdoor if possible". When
one does not go out, or comes back undecided, the page tells you why:

| The console says | Why | Fix |
|---|---|---|
| "This phone hasn't taken a call yet…" (`sim_unknown`) | Bageo does not know the country of the phone's SIM, so an errand could dial from the wrong country's SIM. | Type the SIM's phone number under *System → Phones*, or set `SIM_COUNTRY` on the Bluetooth computer ([Phone setup](PHONE.md#good-to-know)). |
| "No phone has a SIM that can call this country's numbers." (`no_sim_for_country`) | Errands dial only from a SIM of the number's country. A number without `+` counts as one of your country. | Write numbers abroad with `+` and the country code. |
| "The AI can't call this number" (`not_allowed`) | Emergency numbers, short codes (7 digits or fewer) and premium-rate numbers are never dialed. Neither are numbers written with an international prefix (`00…`, `011…`) instead of `+`. | Use the `+` form, or call it yourself. |
| "No AI phone is available right now" (`no_phone`) | No phone is connected, or every phone is in normal phone mode. | Switch one back on *System → Phones*. |
| The errand stays *Queued* | The phone is on a call, or an earlier call to the same number (an errand or an appointment call-back) is still running. | Wait: it dials by itself when free. It gives up after 10 minutes (`phone_busy`), or after 25 minutes behind the same number (`number_busy`). |
| "A voice menu that needs key presses" (`keypad_menu`) | The AI cannot press keys, and waits up to 90 seconds for a person. | Call such numbers yourself, or find a direct number. |
| "Carrier message: …" (`carrier_message`) | The carrier answered with a recording (number not in service, phone off). | Check the number. |
| The AI's question never showed up | Questions show only while you watch that errand (its live view open within the last 6 seconds), and the AI waits 40 seconds. Otherwise it notes the other side's options and ends politely. | The result is *Your decision needed*, with the options to pick from for a call back. |
| *Your decision* although the AI booked something | The AI settled a question you did not answer. | Check the details; if it does not suit you, **Call again** with what to change. |

## Notifications

| Symptom | Why | Fix |
|---|---|---|
| *Send test* shows an error | The text is the service's own answer, usually a wrong token, chat ID or URL. | Follow the steps for that service in [Notifications](NOTIFICATIONS.md). |
| Test works, but no message after calls | *Send notifications* is off, or the channel is disabled. | Switch both on. |
| No summary, only the caller's first words | There is no summary service. | Set the Gemini key (used by default), or your own service under *Summary & calendar*. The terminal shows the reason if the service refused. |
| Links in notifications do not open | *Address in notifications* (*Settings → Notifications*) is not reachable from your phone, or Bageo listens only on this computer (`AGENT_HOST=127.0.0.1`). | See [Using the console from other devices](SETTINGS.md#using-the-console-from-other-devices). |
| The calendar link opens the wrong calendar | The first press of **Add to calendar** asks which calendar to use and remembers it. | Press **Pick another calendar** next to it. Apple Calendar and `.ics` links need *Address in notifications*. |

## FAQ

**`bageo.local` does not open?**
Check, in this order:

- The phone or computer is on the same Wi-Fi as the computer running Bageo (a guest network usually is not).
- From another device, a console password must be set first on the computer running Bageo; until then you only see "Set a password first".
- On Windows, answer the "Allow access" prompt with *Private networks*. If you chose otherwise, open *Windows Security → Firewall & network protection → Allow an app through firewall* and tick *Private* for Bageo's `agent.exe`.
- On a VPN that sends all traffic through the tunnel, turn the VPN off and try again, or see [Using the console from other devices](SETTINGS.md#using-the-console-from-other-devices).
- Opening it by your own domain, a reverse proxy or a VPN name shows "This address can't open the console": enter that address under *Address in notifications*.
- When another program holds port 80, use `http://bageo.local:8787/`; *System → About* shows the address that works.
- Some devices cannot open `.local` addresses: use the IP address or the QR code under *System → About*.

**Does it work on an iPhone, or on Android without root?**
Yes. It answers over Bluetooth, so the phone needs no app and no root. You need
a Linux computer with Bluetooth near the phone: see [Phone setup](PHONE.md).

**Will it work on my phone?**
Any iPhone or Android phone that can pair with a car kit should. It has been
tested with real calls on a few models; reports help. See [Phone setup](PHONE.md).

**Can I keep using the phone normally?**
Yes. It stays an ordinary phone: calls you dial yourself, or answer on the
phone, are never taken over. To have no AI on incoming calls for a while, switch
it to [normal phone mode](PHONE.md#normal-phone-mode) in the web console, and back
when you are done.

**What does it cost?**
Bageo is free for personal and open-source use under `AGPL-3.0-only`
(closed-source commercial use requires a
[Commercial License](../../COMMERCIAL_LICENSE.md)). What you pay for:

- Gemini Live audio: Google bills it by use, for incoming calls, AI call-backs
  and AI errands alike.
- The summary: a small text request to Gemini's lowest-cost model.
- A phone plan that receives calls (and places the calls the AI makes for
  you), and electricity.

Voice previews and the blocked-number announcement come prerecorded and cost
nothing.

**Can it run without Google, fully offline?**
Not in the supported mode: the natural, low-delay voice is Gemini Live. The
summary can use a local model (Ollama), see
[Settings](SETTINGS.md#summary--calendar). A local voice pipeline is on the
[roadmap](ROADMAP.md).

**Can it place calls?**
Yes. **AI errands**: you write on the *Errands* page what to get done, and the
AI calls, handles it, asks you when unsure and reports back. **AI call-backs**
give a caller your answer to an appointment. Both go out through the phone
over Bluetooth.

**Can it reach me when something is urgent?**
Yes, with *Forward to your phone* under *Settings → How it answers*. Pick
whether to **send calls straight to your phone** (via your carrier's
forward-when-busy) or have the **AI answer and alert you if urgent** (an instant
notification, and the AI tells the caller you will call back), and tick
**Who can be forwarded** (*Whitelist*, *Other contacts*, or *Numbers not in your
contacts*).

**Several phones, several numbers?**
Not at the same time: the Bluetooth program takes one phone at a time. A
dual-SIM phone works; see [Phone setup](PHONE.md#good-to-know).

**Where are my recordings, and how do I delete them?**
In `data/calls/<date>/`. They are deleted after *Days to keep records*
(*Settings → Advanced*, default 15 days); delete the folder to remove everything at once.

**Is the caller told it is an AI?**
With the *AI assistant* style, in the first sentence. With *As me*, it speaks as
you without saying it is an AI. Whether the second is allowed where you live is
yours to check: see [Privacy, safety and the law](PRIVACY_AND_LEGAL.md).

## Reporting a problem

Open an issue with your system (Windows, macOS or Linux), the Bluetooth
computer's system, phone model and Android or iOS version, the Bageo version
(bottom of the *System* page), what you did, what you expected, and the relevant
lines of Bageo's terminal output and the Bluetooth log. Remove phone numbers and
keys first, and never post `settings.json`.

<!-- SPDX-FileCopyrightText: h4lo-void <https://github.com/h4lo-void/bageo-call> -->
