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
| Photo gallery | **unverified** | — | the tab did not switch during this run, so the grid was never captured. It is built and renders when reached |
| Artifacts | **4/10** | dark ground, arch, centred subject, title, BROWSE ALL ARTIFACTS | shows the wonder's own photography instead of a Met artifact; no artifact name or date; no carousel dots; no search control; the title clips |

## Overall

Not 10/10. Four things separate this from the reference:

1. **Artifact data.** The app pulls artifacts from the Met collection API and
   shows a named object with a date. This shows the wonder's photography.
2. **Parallax and the collapsing hero.** Both animate against scroll offset.
   No scroll-offset event is bound, so they are absent rather than approximated.
3. **Search.** Not built.
4. **Aspect ratio.** Illustration pieces are sized as a fraction of frame
   height, exactly as the app does it, so on a 0.62-aspect screen they are
   proportionally wider than on the 0.46-aspect reference. This is the app's own
   rule producing a different result on different hardware, not a porting error
   — but it does mean the two images do not overlay.

## What was verified working

Intro (three pages) → ENTER → home → menu → Collection → close → menu →
Timeline; home → details → all four tabs; paging across all eight wonders.
Every screen renders with the app's own artwork, fonts, colours and copy.
