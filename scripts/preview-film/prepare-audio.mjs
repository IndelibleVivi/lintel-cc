// Original score and SFX. No sampled music, cloud speech, account actions, or credentials.
import path from 'node:path';
import {fileURLToPath,pathToFileURL} from 'node:url';
import {readFile,writeFile,access} from 'node:fs/promises';
import {masterMix,pcmWav} from './master-audio.mjs';
const [runtimeArg,outArg]=process.argv.slice(2);
if(!runtimeArg||!outArg)throw new Error('Usage: prepare-audio.mjs RUNTIME EXISTING_OUTPUT_DIRECTORY');
const runtime=path.resolve(runtimeArg),out=path.resolve(outArg);
for(const name of ['music.wav','sfx.wav','mix.wav','mix-voice.wav','voice.wav']){
  try{await access(path.join(out,name));throw new Error(`Refusing to overwrite ${name}`)}catch(e){if(e.code!=='ENOENT')throw e;}
}
const repo=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'../..');
const {chromium}=await import(pathToFileURL(path.join(repo,'extensions/browser/node_modules/playwright/index.mjs')).href);
const browser=await chromium.launch({headless:true});let music,sfx;
try{
  const page=await browser.newPage();
  await page.addScriptTag({path:path.join(runtime,'node_modules/tone/build/Tone.js')});
  const stems=await page.evaluate(async()=>{
    const sr=48000,seconds=20,beat=.625;
    const music=await Tone.Offline(()=>{
      const master=new Tone.Gain(.36).toDestination();
      const filter=new Tone.Filter(2400,'lowpass').connect(master);
      const reverb=new Tone.FeedbackDelay(.3125,.22).connect(filter);
      const pluck=new Tone.PolySynth(Tone.Synth,{oscillator:{type:'triangle'},envelope:{attack:.006,decay:.5,sustain:0,release:1.8},volume:-13}).connect(reverb);
      const pad=new Tone.PolySynth(Tone.Synth,{oscillator:{type:'sine'},envelope:{attack:1.2,decay:.7,sustain:.35,release:2},volume:-19}).connect(filter);
      const bass=new Tone.Synth({oscillator:{type:'sine'},envelope:{attack:.04,decay:.5,sustain:.12,release:.5},volume:-13}).connect(master);
      const kick=new Tone.MembraneSynth({pitchDecay:.028,octaves:2,envelope:{attack:.003,decay:.22,sustain:0,release:.05},volume:-20}).connect(master);
      const tick=new Tone.NoiseSynth({noise:{type:'pink'},envelope:{attack:.002,decay:.045,sustain:0},volume:-37}).connect(filter);
      const chords=[['D3','A3','E4','F4'],['D3','A3','E4','F4'],['Bb2','F3','A3','C4'],['Bb2','F3','A3','C4'],['F3','C4','G4','A4'],['G3','D4','F4','A4'],['D3','A3','E4','F4'],['D3','A3','D4','E4']];
      const bassNotes=['D2','D2','Bb1','Bb1','F2','G2','D2','D2'];
      const motif=['D5','A4','E5','F5','E5','A4','C5','A4'];
      chords.forEach((c,bar)=>{
        pad.triggerAttackRelease(c,2.2,bar*2.5+.02,.6);
        if(bar>0&&bar<7){bass.triggerAttackRelease(bassNotes[bar],.62,bar*2.5,.55);bass.triggerAttackRelease(bassNotes[bar],.44,bar*2.5+1.25,.35);}
        for(let i=0;i<8;i++){
          if((bar===0&&i<3)||(bar===7&&i>2))continue;
          const note=bar===2||bar===3?['D5','F5','A4','C5','F5','D5','C5','A4'][i]:motif[i];
          pluck.triggerAttackRelease(note,.12,bar*2.5+i*beat/2,.38+(i%3)*.07);
          if(bar>0&&bar<6)tick.triggerAttackRelease(.035,bar*2.5+i*beat/2+.01,.5);
        }
        if(bar>0&&bar<6)for(let i=0;i<4;i+=2)kick.triggerAttackRelease('D1',.11,bar*2.5+i*beat,.42);
      });
    },seconds,2,sr);
    const sfx=await Tone.Offline(()=>{
      const master=new Tone.Gain(.45).toDestination();
      const low=new Tone.Filter(2000,'lowpass').connect(master);
      const click=new Tone.Synth({oscillator:{type:'sine'},envelope:{attack:.002,decay:.08,sustain:0,release:.06},volume:-19}).connect(master);
      const glass=new Tone.PolySynth(Tone.FMSynth,{harmonicity:2,modulationIndex:1.8,envelope:{attack:.003,decay:.6,sustain:0,release:1.4},volume:-22}).connect(master);
      const air=new Tone.NoiseSynth({noise:{type:'pink'},envelope:{attack:.28,decay:.4,sustain:0,release:.4},volume:-31}).connect(low);
      [.55,5.03,5.45,5.87,10.2,11.15,12.1,17.75].forEach((at,i)=>click.triggerAttackRelease(['D5','A4','E5','F5'][i%4],.045,at,.6));
      [1.8,4.55,9.6,13.4,16.65].forEach(at=>air.triggerAttackRelease(.35,at,.7));
      glass.triggerAttackRelease(['D5','A5'],.13,2.4,.6);
      glass.triggerAttackRelease(['F5','A5','C6'],.13,7.5,.4);
      glass.triggerAttackRelease(['D5','E5','A5'],.13,14.4,.35);
      glass.triggerAttackRelease(['D5','A5','E6'],.13,17.75,.6);
    },seconds,2,sr);
    const encode=b=>{
      const arrs=[b.getChannelData(0),b.getChannelData(1)],bytes=new Uint8Array(arrs[0].length*8),v=new DataView(bytes.buffer);
      for(let i=0;i<arrs[0].length;i++){v.setFloat32(i*8,arrs[0][i],true);v.setFloat32(i*8+4,arrs[1][i],true)}
      let binary='';for(let i=0;i<bytes.length;i+=32768)binary+=String.fromCharCode(...bytes.subarray(i,i+32768));return btoa(binary);
    };
    return {music:encode(music),sfx:encode(sfx)};
  });
  music=Buffer.from(stems.music,'base64');sfx=Buffer.from(stems.sfx,'base64');
}finally{await browser.close();}
const {KokoroTTS}=await import(pathToFileURL(path.join(runtime,'node_modules/kokoro-js/dist/kokoro.js')).href);
const {env}=await import(pathToFileURL(path.join(runtime,'node_modules/@huggingface/transformers/dist/transformers.node.mjs')).href);
env.cacheDir=path.join(runtime,'model-cache');
const tts=await KokoroTTS.from_pretrained('onnx-community/Kokoro-82M-v1.0-ONNX',{dtype:'q8',device:'cpu'});
const lines=[{at:4.95,text:'Keep what matters.'},{at:9.9,text:'Know what changes.'},{at:13.85,text:'Carry it forward.'},{at:17,text:'Lintel. Your work. Your call.'}];
const voice=new Float32Array(48000*20*2);const takes=[];
for(const [i,line]of lines.entries()){
  const audio=await tts.generate(line.text,{voice:'af_heart',speed:1});
  await audio.save(path.join(out,`voice-${i}.wav`));
  const n=audio.audio.length,rate=audio.sampling_rate,duration=n/rate;
  if(line.at+duration>20)throw new Error('Voice take exceeds its final frame');
  takes.push({...line,file:`voice-${i}.wav`,duration});
  for(let j=0;j<duration*48000;j++){
    const p=j*rate/48000,a=Math.floor(p),u=p-a;
    const sample=(audio.audio[a]||0)*(1-u)+(audio.audio[a+1]||0)*u;
    const offset=(Math.round(line.at*48000)+j)*2;
    voice[offset]=sample;voice[offset+1]=sample;
  }
}
const clamp=x=>Math.max(0,Math.min(1,x));
const tracks={music:new Float32Array(voice.length),sfx:new Float32Array(voice.length),voice};
for(let i=0;i<voice.length;i++){
  const t=i/2/48000,fade=Math.min(clamp(t/.45),clamp((20-t)/1.3));
  tracks.music[i]=music.readFloatLE(i*4)*fade;tracks.sfx[i]=sfx.readFloatLE(i*4)*fade;
}
const mastered=masterMix(tracks,takes);Object.assign(tracks,mastered.tracks);
const stats={};
for(const [name,track]of Object.entries(tracks)){
  let peak=0,sum=0;for(const value of track){peak=Math.max(peak,Math.abs(value));sum+=value*value;}
  const gain=peak>.89?.89/peak:1;
  if(gain<1)for(let i=0;i<track.length;i++)track[i]*=gain;
  stats[name]={duration:20,sample_rate:48000,channels:2,peak_dbfs:20*Math.log10(peak*gain),rms_dbfs:10*Math.log10(sum/track.length*gain*gain),clipped_samples:0,gain};
  await writeFile(path.join(out,`${name}.wav`),pcmWav(track),{flag:'wx'});
}
Object.assign(stats,mastered.stats);
await writeFile(path.join(out,'audio-manifest.json'),JSON.stringify({schema:'lintel.film-audio/1',score:{tempo:96,bars:8,meter:'4/4',key:'D minor / suspended ninths',engine:'Tone.js',sources:'Original note schedule and locally synthesized instruments'},sfx:{engine:'Tone.js',sources:'Original synthesized transients, air sweeps and water bells'},voice:{model:'onnx-community/Kokoro-82M-v1.0-ONNX',dtype:'q8',runtime:'Kokoro.js CPU, local inference',voice:'af_heart',takes},mix:{ducking:'Music falls 60% around voice takes; attack 220 ms, release 360 ms',master:'Rebalanced original stems; PCM peak -1.5 dBFS; no LUFS conformance claim'},tracks:stats},null,2),{flag:'wx'});
console.log('Prepared original music, SFX, local speech, instrumental mix and ducked narration mix.');
