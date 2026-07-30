import { AbilityConstant, UIAbility, Want } from '@kit.AbilityKit';
import { window } from '@kit.ArkUI';
import { webview } from '@kit.ArkWeb';

/** Stock ability. All UI lives in Rust — see pages/Index.ets. */
export default class EntryAbility extends UIAbility {
  /**
   * Register the `splash://` scheme before anything can create a Web.
   *
   * This is the one part of the asset protocol that cannot be done later.
   * `customizeSchemes` is static and configures the web engine itself, so it
   * has to run before the engine initialises — once a `Web` exists the
   * registration is too late, and the failure is not an exception at the call
   * site but a page that loads nothing.
   *
   * `onCreate` is the earliest hook the ability has, and it runs before
   * `onWindowStageCreate` loads the page that builds the components.
   *
   * The flags are not decoration:
   *   - isStandard  gives the scheme normal URL semantics, so `/app.css` in a
   *                 document resolves against its origin. Without it a relative
   *                 subresource does not resolve at all — which is the exact
   *                 failure this protocol exists to fix.
   *   - isSecure    marks the origin trustworthy, so the page is not handled as
   *                 insecure content.
   *   - CORS/fetch  let the page's own `fetch()` reach the scheme, which the
   *                 bundle's 404 check needs.
   *
   * `isCspBypassing` is deliberately not set: it is the one flag here that
   * weakens the page rather than describing the scheme.
   */
  onCreate(_want: Want, _launchParam: AbilityConstant.LaunchParam): void {
    try {
      webview.WebviewController.customizeSchemes([{
        schemeName: 'splash',
        isStandard: true,
        isSecure: true,
        isSupportCORS: true,
        isSupportFetch: true,
      }]);
      console.info('SPLASHSCHEME registered splash://');
    } catch (e) {
      // 17100020 is "failed to register custom schemes". Worth distinguishing
      // from a silent no-op: everything downstream of it fails as a blank page
      // with no other symptom.
      console.error('SPLASHSCHEME registration failed: ' + JSON.stringify(e));
    }
  }

  onWindowStageCreate(windowStage: window.WindowStage): void {
    windowStage.loadContent('pages/Index');
  }

  onWindowStageDestroy(): void {}
  onForeground(): void {}
  onBackground(): void {}
  onDestroy(): void {}
}
