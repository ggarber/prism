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

class VideoRenderer {
  constructor(canvas) {
    this.canvas = canvas;
    this.ctx = canvas.getContext("2d");
    this.pendingFrames = [];
    this.baseTime = 0;
  }

  render(frame) {
    pendingFrames.push(frame);
    if (this.underflow) setTimeout(this.renderFrame.bind(this), 0);
  }

  calculateTimeUntilNextFrame(timestamp) {
    if (this.baseTime === 0) this.baseTime = performance.now();
    const mediaTime = performance.now() - baseTime;
    return Math.max(0, timestamp / 1000 - mediaTime);
  }
  
  async renderFrame() {
    this.underflow = pendingFrames.length === 0;
    if (this.underflow) return;
  
    console.log('renderFrame');
    const frame = this.pendingFrames.shift();
  
    // Based on the frame's timestamp calculate how much of real time waiting
    // is needed before showing the next frame.
    const timeUntilNextFrame = this.calculateTimeUntilNextFrame(frame.timestamp);
    await new Promise((r) => {
      setTimeout(r, timeUntilNextFrame);
    });
    ctx.drawImage(frame, 0, 0);
    frame.close();
  
    // Immediately schedule rendering of the next frame
    setTimeout(this.renderFrame.bind(this), 0);
  }
}