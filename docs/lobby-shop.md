# Mira's equipment shop

Mira stands behind the gold-lit counter on the right side of the lobby. Walk
near the counter and press E when the shop prompt appears. The menu pauses the
world and shows stock, item details, inventory space and your Meseta balance.

- Click **Buy stock** or **Sell items** to change tabs.
- Click an item, or use Up/Down, to select it. Long lists page with selection.
- Click the clearly labeled Buy/Sell button or press Enter to trade.
- Escape or **Leave shop** returns to play. Tab / I switches to inventory.

Purchased weapons appear as new inventory instances with an empty PA slot.
Use Tab / I to equip a weapon and assign a disc. Each PA is unique: buying an
already-owned disc is rejected without charging money. Selling a PA removes
it from its slot; selling a weapon returns its slotted PA to the bag. The currently
equipped weapon cannot be sold: equip another one first.

## Currency and stock

New characters begin with **300 Meseta** as well as the existing starter bag.
Clearing all three trial waves awards **300 Meseta**, once per completed trial.
Retry, individual wave kills, extraction and attacks after victory do not award
extra money. Repeat trials to earn more. Wallet capacity is 999,999 Meseta.

| Item | Buy | Sell |
| --- | --- | --- |
| Training Sword | 60 | 30 |
| Hunter Saber | 180 | 90 |
| Photon Edge | 320 | 160 |
| Photon Sweep disc | 120 | 60 |
| Rising Cleave disc | 160 | 80 |
| Piercing Thrust disc | 100 | 50 |
| Recruit Partisan | 180 | 90 |
| Assault Thrust disc (Partisan) | 100 | 50 |
| Recruit Knuckles | 160 | 80 |
| Meteor Fist disc (Knuckles) | 80 | 40 |

Trades require a living Hunter within 2.7 units of the counter in the lobby.
Ownership, money, bag capacity, equipped-weapon protection and wallet limits are
validated before mutation. Failed trades preserve both inventory and wallet.
Shop prices are fixed prototype values; there is no real-money purchasing.

Currency, purchases and inventory [autosave between launches](saving.md).
Returning to the lobby keeps your gear and money. Restocking rules and item
rarities are future work. Mira, her counter display and stock are original art.

## Validation

Core tests cover successful purchases, insufficient money, full bags, duplicate
discs, invalid ownership, sale protection, disc return/unslotting, wallet limits,
lobby proximity and exactly-once quest rewards. `PSP_SHOP_SMOKE=1` runs the ordinary
game input through a counter visit, purchase/sale/repurchase, three-wave trial,
extraction, another purchase using the reward and equipping that new weapon.
`PSP_GAME_PREVIEW=shop` and `PSP_GAME_PREVIEW=keeper` support local framebuffer
captures with `PSP_GAME_CAPTURE=/path/to/frame.png`.
