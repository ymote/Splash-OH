# Wonderous — visual comparison against the reference

Scored against `web/screenshots/mobile1..4.png` from
gskinnerTeam/flutter-wonderous-app, captured on a HUAWEI Pura X (VDE-AL10,
HarmonyOS 6.1, 1320×2120, ratio 3). Reference on the left of each pair in
`wonderous-vqa.png`, this app on the right.

The scores are mine, from looking at the pairs. They are not a model's output
and not a pixel metric.

| screen | score | what matches | what does not |
|---|---|---|---|
| Home (Christ the Redeemer) | **9/10** | the same illustration pieces in the app's own three layers — background, clouds, wonder, foreground — its palette, Yeseva One title with the small italic article beside the second line, eight dots, menu button, chevron; pages by swipe | the pieces are sized by `heightFactor × frameHeight`, the app's own rule, so on a 0.62-aspect screen they come out proportionally larger than on the 0.46-aspect reference |
| Editorial | **9/10** | hero photograph with parallax at half the scroll rate and a gradient scrim that fades with the title, the app's own body copy, arc section labels with their icons, pull-quotes, callouts, the video still with its play button, and it scrolls | the reference is scrolled to a different point, so the pair is not aligned; no scroll-driven transitions between sections |
| Photo gallery | **9/10** | the app's 5×5 wall over its own Unsplash collection — all 24 photographs per wonder, from the ids in `unsplash_photo_data.dart` — cells two thirds of the screen wide and half of it tall, the selected one centred with a 70% scrim over the rest, panned by swipe in four directions, and the pan is tweened at the app's own duration and curve | the app swipes eight ways where this takes four; the fifth-row wrap repeats the first photograph, as the app's own `_initPhotoIds` does |
| Artifacts | **9/10** | the piece's own photograph blurred behind a black wash, the pale disc rising across the middle, the piece in a capsule outlined in off-white with its neighbours as circles either side, near-black name and date on the pale ground, page dots, ARTIFACTS with the search button, BROWSE ALL ARTIFACTS; swipes and taps both page it | no collapsing animation between carousel items; the search screen it opens filters the 32 artifacts that ship rather than querying the Met live |

| screen | state | notes |
|---|---|---|
| Search | **built** | the wonder's own suggestion words as chips, chip selection, live result count, a grid of the real Met artifacts. No text entry — the chips are the whole input |
| Menu, Collection, Timeline, Events, Intro | **built** | all eight wonders, the global timeline from 2600 BCE to 1931 CE, each wonder's own dated events, the intro's three pages |

## What is still not the app

1. **Motion.** The parallax is live and driven by the real scroll offset, and
   the gallery pan is tweened through ArkUI's own `animateTo`
   (`wonderous-gallery-tween.png` catches it mid-slide). The rest still changes
   state instantly: Wonderous also animates the carousel collapse, the page
   transitions and the wonder cross-fade. Those all change *content*, so they
   need two trees cross-faded rather than one tree moved.
2. **Aspect ratio.** Illustration pieces are sized as a fraction of frame
   height, exactly as the app does it, so on a 0.62-aspect screen they are
   proportionally larger than on the 0.46-aspect reference. This is the app's
   own rule producing a different result on different hardware, not a porting
   error — but it does mean the two images do not overlay.
3. **Live data.** The Met artifact search and the Unsplash collections are
   fetched at build time and shipped, where the app queries them at runtime.
   The content is the app's own; the freshness is not.

## What was verified on the device

Intro (three pages) → ENTER → home → menu → any wonder → details → all four
tabs → search → back; home → Collection → close; home → Timeline → close.
Paging across all eight wonders by swipe and by tap. The photo wall panned in
all four directions by swipe and by tapping its edges. The artifact carousel
paged both ways. The editorial scrolled, with the hero measured moving at
0.51× the article over it. The gallery's tween was confirmed by stretching it
to 2.5 s and catching three frames of the slide, then putting it back.

Every screen renders with the app's own artwork, fonts, colours, copy, dates
and artifacts.
