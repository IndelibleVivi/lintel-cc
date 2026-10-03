import assert from 'node:assert/strict';
import {readFile, writeFile} from 'node:fs/promises';
import path from 'node:path';

// Headless CDP file drops do not populate WebContents' native dropped path.
// Use Chromium's own chrome://extensions reload-error recovery instead. It
// creates a browser-owned retryGuid, then loadUnpacked performs the persistent
// install. The caller must pass a disposable copy of the synthetic fixture.
// https://chromium.googlesource.com/chromium/src/+/main/chrome/browser/extensions/api/developer_private/developer_private_functions.cc
export async function persistSyntheticExtension(context, directory, extensionId) {
  const manager = await context.newPage();
  try {
    await manager.goto('chrome://extensions');
    const initial = await manager.evaluate(id => chrome.developerPrivate.getExtensionInfo(id), extensionId);
    assert.equal(initial.name, 'Lintel — SYNTHETIC TEST ONLY');
    assert.equal(initial.location, 'UNPACKED');
    await manager.locator('#devMode').click();
    assert.equal((await manager.evaluate(() => chrome.developerPrivate.getProfileConfiguration())).inDeveloperMode, true);

    const manifestPath = path.join(directory, 'manifest.json');
    const originalManifest = await readFile(manifestPath);
    let loadError;
    try {
      await writeFile(manifestPath, '{');
      loadError = await manager.evaluate(id => chrome.developerPrivate.reload(id, {
        failQuietly: true, populateErrorForUnpacked: true,
      }), extensionId);
    } finally {
      await writeFile(manifestPath, originalManifest);
    }
    assert.equal(typeof loadError?.retryGuid, 'string', JSON.stringify(loadError));
    assert.match(loadError.error, /Manifest is not valid JSON/);
    const workerReady = context.waitForEvent('serviceworker', {
      timeout: 15000,
      predicate: worker => worker.url() === `chrome-extension://${extensionId}/background.js`,
    });
    // Keep the event waiter handled even if the native installer rejects.
    workerReady.catch(() => {});
    const result = await manager.evaluate(retryGuid => chrome.developerPrivate.loadUnpacked({
      retryGuid, failQuietly: true, populateError: true,
    }), loadError.retryGuid);
    assert.equal(result, undefined, JSON.stringify(result));
    const installed = await manager.evaluate(id => chrome.developerPrivate.getExtensionInfo(id), extensionId);
    assert.equal(installed.state, 'ENABLED');
    const worker = await workerReady;
    const installation = {
      mechanism: 'chrome://extensions native reload-error recovery and loadUnpacked retryGuid',
      location: installed.location,
      state: installed.state,
      extensionId,
      manifestRestoredByteForByte: (await readFile(manifestPath)).equals(originalManifest),
    };
    assert.equal(installation.manifestRestoredByteForByte, true);
    return {worker, installation};
  } finally {
    await manager.close();
  }
}
