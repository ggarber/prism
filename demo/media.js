let audioCtx = null;

async function loadAudioContext() {
  if (audioCtx) {
    return audioCtx
  }
  audioCtx = new Promise((resolve, reject) => {
    const ctx = new AudioContext();
    ctx.audioWorklet.addModule('media.js').then(() => {
      resolve(ctx);
    }).catch(reject);
  });
}
