# Tinkerway

A home base for a maker, or a small team.

You type whatever is in your head. The app keeps the context, turns it into an experiment or a project, and hands the next piece to a person or to a tool you already use. The home base is a folder of files you own.

## App

Capture a thought (Demo v1): type at the bottom, Send puts it in the feed above, tap a row to read it. Capture only — no folders, tags, or replies. The words are sealed in an encrypted `.tw` vault (OS keystore master key). Desktop shell under [`apps/tinkerway-tauri`](apps/tinkerway-tauri/) ([Tauri 2](https://v2.tauri.app/) + system webview + plain HTML/CSS/JS); vault IO in [`crates/tinkerway-vault`](crates/tinkerway-vault/) with no UI types. Run: [`apps/tinkerway-tauri/README.md`](apps/tinkerway-tauri/README.md). Norms: [`docs/`](docs/).

```bash
mise install
MBX_DISABLE=1 cargo run -p tinkerway-tauri
```

## Housekeeping

Secrets tooling (mise / hk / betterleaks): [`docs/dev-setup.md`](docs/dev-setup.md).

The app demands the source be public because it is more personal with all the notes, context, and everything.

So it can't be a black box people trust blindly.

That said,

1. This is very much an experimental software where I tinker with my curiosity so things may change rapidly but making sure data is always yours backed up securely to whatver source you choose.

2. Future version might become closed source if it gets out of my hand to manage (security or any other problem that I can't manage and affect living peacefully)

3. Very opinionated so I may not merge PRs randomly. I still hope many talented people than me contribute so I won't randomly discard randomly as well. All this to say please don't have high hopes when you contribute. I know I know, it doesn't feel exciting but hey, I gotta do it for my own sanity.

4. This is a running list of disclaimers, to protect against increased work load and to keep some sanity for myself while managing a project that is out there in the public.
