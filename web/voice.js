// The car's sound, made in the page from what the engine says is happening each tick: how fast the
// engine turns and how hard it works, the turbo, the tyres, the wind and a siren. The engine only
// hands over eight numbers (see wtk_sound); everything you hear is built here with Web Audio.

export function voice(audio, out) {
  const a = audio, now = () => a.currentTime;
  const glide = (param, value, time = 0.03) => param.setTargetAtTime(value, now(), time);

  // A couple of seconds of noise, looped: the raw stuff of intake roar, wind, tyres and valves.
  const hiss = a.createBuffer(1, a.sampleRate * 2, a.sampleRate);
  const data = hiss.getChannelData(0);
  for (let i = 0; i < data.length; i++) data[i] = Math.random() * 2 - 1;
  const noise = () => { const n = a.createBufferSource(); n.buffer = hiss; n.loop = true; n.start(); return n; };
  const gain = (to, value = 0) => { const g = a.createGain(); g.gain.value = value; g.connect(to); return g; };
  const filter = (type, freq, q, to) => { const f = a.createBiquadFilter(); f.type = type; f.frequency.value = freq; f.Q.value = q; f.connect(to); return f; };

  // The engine: a buzz at the firing rate and a growl an octave down, roughened and dulled by a
  // filter that opens up as it works harder.
  const engine = gain(out);
  const tone = filter("lowpass", 600, 3, engine);
  const rough = a.createWaveShaper();
  const curve = new Float32Array(1024);
  for (let i = 0; i < curve.length; i++) { const v = (i / (curve.length - 1)) * 2 - 1; curve[i] = Math.tanh(v * 3); }
  rough.curve = curve;
  rough.connect(tone);
  const fire = a.createOscillator(); fire.type = "sawtooth";
  const growl = a.createOscillator(); growl.type = "square";
  const growlLevel = gain(rough, 0.55);
  fire.connect(gain(rough, 0.7)); growl.connect(growlLevel);
  fire.start(); growl.start();
  // Air through the intake: noise around the firing rate, louder with the throttle open.
  const intake = gain(out);
  const intakeBand = filter("bandpass", 400, 1.2, intake);
  noise().connect(intakeBand);
  // The turbo: a thin whistle that climbs as it spools.
  const turbo = gain(out);
  const whistle = a.createOscillator(); whistle.type = "sine"; whistle.connect(turbo); whistle.start();
  // Tyres at their limit, and the wind.
  const tyres = gain(out);
  const squealBand = filter("bandpass", 2300, 7, tyres);
  noise().connect(squealBand);
  const wind = gain(out);
  noise().connect(filter("lowpass", 450, 0.7, wind));
  // The siren: a wail up and down.
  const siren = gain(out);
  const wail = a.createOscillator(); wail.type = "triangle"; wail.connect(siren); wail.start();

  let blowoffs = -1;
  // The valve letting go: a burst of air that falls away.
  const blowoff = () => {
    const n = a.createBufferSource(); n.buffer = hiss;
    const band = a.createBiquadFilter(); band.type = "bandpass"; band.Q.value = 1.4;
    const g = a.createGain(), t = now();
    band.frequency.setValueAtTime(2400, t); band.frequency.exponentialRampToValueAtTime(700, t + 0.45);
    g.gain.setValueAtTime(0.22, t); g.gain.exponentialRampToValueAtTime(0.001, t + 0.45);
    n.connect(band).connect(g).connect(out);
    n.start(t); n.stop(t + 0.5);
  };

  // One tick's numbers: rpm, load, boost, cylinders, squeal, blow-offs, siren, wind.
  return (s) => {
    const [rpm, load, boost, cylinders, squeal, blown, sirenOn, speed] = s;
    const on = rpm > 0;
    // Four strokes: each cylinder fires every other turn.
    const firing = Math.max(on ? rpm : 0, 600) / 60 * Math.max(cylinders, 1) / 2;
    // A little unevenness, as a real engine has, more of it at idle.
    const lump = 1 + (Math.random() - 0.5) * (rpm < 1500 ? 0.06 : 0.015);
    glide(fire.frequency, firing * lump, 0.015);
    glide(growl.frequency, firing / 2, 0.015);
    glide(tone.frequency, 350 + load * 2200 + rpm * 0.25);
    glide(engine.gain, on ? 0.05 + load * 0.09 : 0, 0.05);
    glide(intakeBand.frequency, Math.min(200 + firing * 2, 3000));
    glide(intake.gain, on ? load * 0.05 : 0, 0.05);
    glide(whistle.frequency, 1800 + boost * 5200, 0.12);
    glide(turbo.gain, on ? boost * 0.012 : 0, 0.1);
    glide(tyres.gain, squeal * 0.07, 0.04);
    glide(squealBand.frequency, 2100 + Math.random() * 400, 0.02);
    glide(wind.gain, speed * speed * 0.08, 0.2);
    glide(wail.frequency, 720 + 260 * Math.sin(now() * Math.PI * 2 * 0.8), 0.02);
    glide(siren.gain, sirenOn * 0.03, 0.05);
    if (blowoffs >= 0 && blown > blowoffs) blowoff();
    blowoffs = blown;
  };
}
