# tinkerway.ai

Cloudflare Worker. This folder is the site. No build step.

`wrangler.jsonc` in this folder serves these files.

On the create screen:

- Project name: `tinkerway`
- Root directory: `site`
- Build command: empty
- Deploy command: `npx wrangler deploy`
- Preview builds: off
- Cloudflare Access: off

Then add the custom domain `tinkerway.ai` on the Worker.
