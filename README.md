# Friendarazzi

Friendarazzi is a multiplayer social photo-hunting Bingo web application where groups of friends turn hangouts, trips, parties, or everyday weeks into a candid photo-hunting game.

## What it does

- **Lobby & Invitation:** Create a game lobby with configurable grid size (`3×3`, `4×4`, `5×5`, optional center Free Space) and proof visibility mode (`Undercover`, `LiveFeed`, or `BlurredTeaser`), and invite friends via short link (`/join/<invite_code>`).
- **Equal-Distribution Drafting & Starter Decks:** Each player submits custom prompts distributed evenly across other participants—or picks and tweaks prompts from 4 curated Quick-Start decks (*Night Out*, *Urlaub & Festival*, *WG & Zuhause*, *Büro & Uni*).
- **Balanced Board Assembly & Active Hunt:** The backend assembles unique Bingo boards per player ensuring no player ever receives a tile they wrote or one targeting themselves. Snap or upload photo proofs straight from the browser.
- **Victory, Superlatives & 1:1 Timeline:** Completing a row, column, or diagonal crowns the winner, awards a Gold Medal (`gold_medal_count`), computes match superlatives (*Most Wanted*, *Ghost*, *Quickest Draw*, *Sniper*), and reveals a proportional 1:1 game timeline and unlocked photo gallery.
- **Profiles & Photo Memories:** Player profiles showcase Gold Medals, rank titles, custom avatars, and game-grouped *Caught* / *Captured* galleries.

## How you use it

Friendarazzi runs in the browser: open the site, sign in, create or join a lobby, and start hunting. The mobile-friendly web app can be added to a phone's home screen (`site.webmanifest`) so it opens like an app.

### Local development

```bash
mise run dev:server
mise run dev:frontend
```

- `dev:server` runs the Rust API (`friendarazzi-server`) with Mailpit (`SMTP 127.0.0.1:1026`, Web UI `http://127.0.0.1:8034`).
- `dev:frontend` serves the Next.js App Router frontend at `http://localhost:3001` talking to `http://localhost:3000/api`.

## Contributing

Contributions are welcome via issues and pull requests in [`TimerErTim/Friendarazzi`](https://github.com/TimerErTim/Friendarazzi). See [`.github/CONTRIBUTING.md`](.github/CONTRIBUTING.md) and [`docs/vertical-slice.md`](docs/vertical-slice.md) for architecture and verification conventions (`mise run check`, `mise run test`, `mise run build`, `mise run verify`).

## License

[GNU Affero General Public License v3.0](LICENSE).
