import { NodeContent } from '@kit.ArkUI';
import splash from 'libsplash_oh.so';

/**
 * The entire ArkTS surface of this app.
 *
 * It declares one empty slot and hands it to Rust exactly once. Every widget on
 * screen — text, buttons, toggles, sliders, inputs, pickers, containers — is
 * created, configured, laid out and event-wired by native code through the
 * ArkUI NDK. There is no other ArkTS in the app.
 *
 * Why: on the octos-one OHOS port a Rust->ArkTS round trip measured ~1.0 ms,
 * ~70% of it just waiting for the JS event loop to become free. That is
 * survivable for one webview rect per frame; it is not survivable when every
 * widget in a tree pays it. Here the JS thread is never on the path at all.
 */
@Entry
@Component
struct Index {
  private content: NodeContent = new NodeContent();

  aboutToAppear(): void {
    // The one and only ArkTS -> native call in the whole app.
    splash.mount(this.content);
  }

  build() {
    Column() {
      ContentSlot(this.content)
    }
    .width('100%')
    .height('100%')
  }
}
