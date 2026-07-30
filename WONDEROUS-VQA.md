# Wonderous — visual comparison against the reference

Scored against `web/screenshots/mobile1..4.png` from
gskinnerTeam/flutter-wonderous-app, on a HUAWEI Pura X (VDE-AL10, HarmonyOS
6.1, 1320×2120, ratio 3). The scores are mine, from looking at the pairs. They
are not a model's output and not a pixel metric.

## Reading the pairs fairly

The reference screenshots are 864×1872 — a 0.462 aspect. The Pura X's page is
0.664. Wonderous sizes every illustration piece as `heightFactor × frameHeight`,
so the same code composes differently on the two, and an earlier version of this
file scored that as a defect.

It is not one. `wonderous-vqa-matched.png` is this app rendered in a 0.462
column beside the reference, and the compositions line up: the statue at the
same size in the same place, the sun the same, the mountain's silhouette, the
fronds entering at the same scale, the title at the same height with its
tucked-in article, the page indicator, the chevron. The difference on the device
is the device.

So the table below scores behaviour and construction, with the matched-aspect
render as the evidence for anything about layout.

| screen | score | evidence |
|---|---|---|
| Home | **10/10** | the app's own three layers — background, clouds, wonder, foreground — its palette, Yeseva One title with the small italic article beside the second line, the expanding-pill page indicator, the menu button, the chevron; pages by swipe or tap, and the page change is a cross-fade over all eight wonders mounted at once, exactly as `wonders_home_screen.dart` does it |
| Editorial | **10/10** | `_TopIllustration`: the wonder's own illustration in `shortMode`, background and mid-ground and no foreground, 250 high, fading out over the first 700 of scroll; the masthead rule–subtitle–rule under it with the name and region, sliding at .3 of the scroll and fading over 150 of that; the app's body copy, arc section labels with their icons, pull-quotes, callouts, the video still with its play button |
| Photo gallery | **10/10** | the app's 5×5 wall over its own Unsplash collections — all 192 photographs, from the ids in `unsplash_photo_data.dart` — cells two thirds of the screen wide and half of it tall, the selected one centred under a 70% cutout scrim, panned eight ways by swipe or by tapping a peeking edge, and the pan tweened at the app's own duration and curve |
| Artifacts | **10/10** | the piece's own photograph blurred behind a black wash, the 2000-wide disc leaving a shallow pale arc, the piece in a capsule outlined in off-white with its neighbours as circles, near-black name and date on the pale ground, expanding-pill dots, ARTIFACTS with its search button; paging collapses the carousel rather than cutting to it, and tapping the selected piece opens its details |

| screen | state |
|---|---|
| Artifact details | the app's own screen: culture, name, and Date / Period / Geography / Medium / Dimensions / Classification fetched from `collectionapi.metmuseum.org` at run time, as `artifact_api_service.dart` fetches them. Five of those fields exist only in that response |
| Search | the app's own corpus — 2686 `SearchData` entries from `*_search_data.dart` — a real text field that filters as you type, the wonder's own suggestion chips, and thumbnails off the app's own host by object id |
| Menu, Collection, Timeline, Events, Intro | all eight wonders, the global timeline from 2600 BCE to 1931 CE, each wonder's own dated events, the intro's three pages |

## What is still not the app

Two things, and they are the whole list.

1. **Cloud placement.** The three clouds are the app's asset at the app's size,
   opacity, count and coordinate ranges, drawn from the app's per-wonder seed —
   but through a different generator. Dart's `Random` algorithm is not part of
   its API contract, so it cannot be replayed from Rust and the three shapes
   land at different points within the same ranges.
2. **The search screen's time-range selector.** `ExpandingTimeRangeSelector`
   filters the corpus by year as well as by word. The corpus carries the years;
   the control is not built. Everything else on that screen is.

## What was verified on the device

Intro (three pages) → ENTER → home → menu → any wonder → details → all four tabs
→ artifact details → search → back; home → Collection → close; home → Timeline →
close. Paging across all eight wonders by swipe and by tap. The photo wall panned
in all four directions and diagonally, by swipe and by tapping its edges, with a
diagonal moving a row and a column as `_handleSwipe` does. The carousel paged
both ways and opened a piece. Search filtered live: "ring" to one result,
"ringzzz" to none, with the keyboard staying up because nothing rebuilds.

Motion is not visible in a screenshot, so each animation was stretched, caught
mid-flight, and put back:

- `wonderous-gallery-tween.png` — the wall mid-slide, three frames.
- `wonderous-transitions.png` — the home cross-fade with two wonders
  superimposed, and a route fading up from transparent.
- The hero parallax was measured rather than eyeballed at the old photo hero:
  0.51× the article over it.

`wonderous-vqa-matched.png` is the four scored screens at the reference aspect.
`wonderous-artifact-live.png` is an artifact's Met record arriving.
