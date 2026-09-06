# Contributing to IluminOS

Hey, thanks for even considering putting time into this ❤️

IluminOS is a solo learning project. An OS written from scratch in
Rust, `no_std`, running on Limine and QEMU. It's not trying to become
the next Linux, it's not trying to be POSIX compliant, it's just a way
to actually understand how a computer boots, draws pixels, reads a
keyboard, and talks to a disk, instead of taking all of that for
granted. If that's the kind of thing you enjoy poking at, you're welcome
here.

If you like the project but don't have time to send code, that's
genuinely fine too. A star on GitHub, sharing it with a friend, or just
mentioning it somewhere goes a long way for a small project like this.

## Found a bug?

Check the [open issues](https://github.com/IbrokhimN/IluminOS/issues)
first to see if someone already ran into it. If not, [open a new
one](https://github.com/IbrokhimN/IluminOS/issues/new) and tell me what
you did to trigger it. Which shell command, which QEMU flags, what you
expected and what actually happened. The more specific, the faster it
gets fixed.

## Fixed something?

Open a PR. Explain what was broken and how your patch fixes it. A
couple of sentences is enough, this isn't a formal process. Link the
issue if there is one. Before you open it, just boot the thing and make
sure the shell still comes up and your change actually does what it's
supposed to. There's no CI watching your back here, so that check is on
you.

## Want to add something new?

Open an issue first and pitch the idea before writing a ton of code.
Saves everyone time if it turns out to not be a great fit. Keep in mind
this project is intentionally small and personal. It's meant to be
something one person can hold in their head, not a full blown production
kernel. If you've got a bigger vision in mind, forking is very much
encouraged, or you might have more fun contributing to something with a
wider scope like [Redox](https://www.redox-os.org/) or
[SerenityOS](https://serenityos.org/).

## Just have a question?

No need to overthink it. Open an issue and ask. Whether it's how do I
get networking working in QEMU or why is the output in English only,
that's a totally valid use of an issue here.
