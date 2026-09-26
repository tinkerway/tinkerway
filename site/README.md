# tinkerway.ai

Cloudflare Worker. This folder is the site. No build step.

`wrangler.jsonc` at the repo root serves this folder.

On the create screen:

- Project name: `tinkerway`
- Build command: empty
- Deploy command: `npx wrangler deploy`
- Preview builds: off
- Cloudflare Access: off

Leave the root directory as the repo root. Then add the custom domain `tinkerway.ai` on the Worker.
