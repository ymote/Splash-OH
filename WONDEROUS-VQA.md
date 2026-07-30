# Wonderous — visual comparison against the reference

Scored against `web/screenshots/mobile1..4.png` from
gskinnerTeam/flutter-wonderous-app, captured on a HUAWEI Pura X (VDE-AL10,
HarmonyOS 6.1, 1320×2120). Reference on the left of each pair in
`wonderous-vqa.png`, this app on the right.

The scores are mine, from looking at the pairs. They are not a model's output
and not a pixel metric.

| screen | score | what matches | what does not |
|---|---|---|---|
| Home (Christ the Redeemer) | **8/10** | illustration layers, palette, Yeseva One title, the small italic article beside the second line, eight dots, menu button, chevron | sun larger and lower, foliage sits differently, cloud bands thinner — the pieces are sized by `heightFactor × frameHeight` and the Pura X is far shorter relative to its width than the reference device |
| Editorial | **6/10** | hero photograph, title and region, real body copy, arc section labels with their icons, video still with play button, scrolls | reference is scrolled to a different point so the pair is not aligned; no collapsing hero, no pull-quote block, callouts not visible in this view |
| Photo gallery | **7/10** | a two-column grid of the wonder's own photography, full bleed, over its colour, reachable from the tab bar | the app pulls a live Unsplash collection; this tiles the four photographs that ship, so the grid repeats |
| Artifacts | **8/10** | the real Met object with its own photograph, name and date — "Guardian Figure, ca. 1919–1885 B.C." — on the arch, over the wonder's colour, with BROWSE ALL ARTIFACTS | no search control in the corner; the app's carousel swipes where this pages by tapping either half |

| Search | **built** | the wonder's own suggestion words as chips, chip selection, result count, a grid of the real artifacts | no text entry — the chips are the whole input; the app queries the Met live and this filters the artifacts that ship |

## Overall

Not 10/10. Four things separate this from the reference:

1. **Parallax and the collapsing hero.** Both animate against scroll offset.
   No scroll-offset event is bound, so they are absent rather than approximated.
3. **Aspect ratio.** Illustration pieces are sized as a fraction of frame
   height, exactly as the app does it, so on a 0.62-aspect screen they are
   proportionally wider than on the 0.46-aspect reference. This is the app's own
   rule producing a different result on different hardware, not a porting error
   — but it does mean the two images do not overlay.

## What was verified working

Intro (three pages) → ENTER → home → menu → Collection → close → menu →
Timeline; home → details → all four tabs; paging across all eight wonders.
Every screen renders with the app's own artwork, fonts, colours and copy.
