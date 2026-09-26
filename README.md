# tinkerway

A home base for a maker, or a small team.

You type whatever is in your head. The app keeps the context, turns it into an experiment or a project, and hands the next piece to a person or to a tool you already use. The home base is a folder of files you own.

## App

Capture a thought as a note: type a line, save it, see it in the list, click to read the body. Desktop app under [`app/`](app/) (Rust + [GPUI](https://gpui.rs)). Notes are plaintext markdown in cwd `.tinkerway-workspace/` for now; an encrypted vault comes later ([`docs/data-and-privacy.md`](docs/data-and-privacy.md)). See [`app/README.md`](app/README.md) to run on Mac (Xcode / Metal required). Norms: [`docs/`](docs/).

## Housekeeping

Secrets stay out of git: local hk + betterleaks (`mise install && hk install --mise`), CI betterleaks on push/PR/daily, and `.cursor/hooks` for agents.

The app demands the source be public because it is more personal with all the notes, context, and everything.

So it can't be a black box people trust blindly.

That said,

1. This is very much an experimental software where I tinker with my curiosity so things may change rapidly but making sure data is always yours backed up securely to whatver source you choose.

2. Future version might become closed source if it gets out of my hand to manage (security or any other problem that I can't manage and affect living peacefully)

3. Very opinionated so I may not merge PRs randomly. I still hope many talented people than me contribute so I won't randomly discard randomly as well. All this to say please don't have high hopes when you contribute. I know I know, it doesn't feel exciting but hey, I gotta do it for my own sanity.

4. This is a running list of disclaimers, to protect against increased work load and to keep some sanity for myself while managing a project that is out there in the public.
