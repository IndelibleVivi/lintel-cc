// One canonical mix for generated stems and later visual revisions. Never overwrite raw takes.
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {readFile,writeFile,mkdir,copyFile} from 'node:fs/promises';
const sr=48000,seconds=20,clamp=x=>Math.max(0,Math.min(1,x));
export function pcmWav(samples){
  const b=Buffer.alloc(44+samples.length*2);b.write('RIFF');b.writeUInt32LE(b.length-8,4);b.write('WAVEfmt ',8);b.writeUInt32LE(16,16);b.writeUInt16LE(1,20);b.writeUInt16LE(2,22);b.writeUInt32LE(sr,24);b.writeUInt32LE(sr*4,28);b.writeUInt16LE(4,32);b.writeUInt16LE(16,34);b.write('data',36);b.writeUInt32LE(samples.length*2,40);
  for(let i=0;i<samples.length;i++)b.writeInt16LE(Math.round(Math.max(-1,Math.min(1,samples[i]))*32767),44+i*2);return b;
}
export function masterMix({music,sfx,voice},takes){
  if([music,sfx,voice].some(x=>x.length!==sr*seconds*2))throw new Error('Expected three complete 20-second stereo stems');
  const tracks={mix:new Float32Array(music.length),'mix-voice':new Float32Array(music.length)},stats={};
  for(let i=0;i<music.length;i++){
    const t=i/2/sr;
    const duck=Math.min(...takes.map(v=>1-.6*Math.min(clamp((t-v.at+.22)/.22),clamp((v.at+v.duration+.36-t)/.36))));
    tracks.mix[i]=music[i]*5+sfx[i]*12;
    tracks['mix-voice'][i]=music[i]*5*duck+sfx[i]*9.6+voice[i]*.78;
  }
  for(const [name,samples]of Object.entries(tracks)){
    let peak=0,sum=0;for(const v of samples){peak=Math.max(peak,Math.abs(v));sum+=v*v;}
    if(!Number.isFinite(peak)||peak===0)throw new Error('Invalid or silent master');
    const gain=.84/peak;for(let i=0;i<samples.length;i++)samples[i]*=gain;
    stats[name]={duration:seconds,sample_rate:sr,channels:2,peak_dbfs:20*Math.log10(.84),rms_dbfs:10*Math.log10(sum/samples.length*gain*gain),clipped_samples:0,gain};
  }
  return {tracks,stats};
}
function readPcm(b){
  if(b.length!==44+sr*seconds*4||b.toString('ascii',0,4)!=='RIFF'||b.toString('ascii',8,16)!=='WAVEfmt '||b.readUInt16LE(20)!==1||b.readUInt16LE(22)!==2||b.readUInt32LE(24)!==sr||b.readUInt16LE(34)!==16||b.toString('ascii',36,40)!=='data')throw new Error('Expected the production engine PCM16 stereo stem');
  return Float32Array.from({length:sr*seconds*2},(_,i)=>b.readInt16LE(44+i*2)/32767);
}
if(process.argv[1]&&path.resolve(process.argv[1])===fileURLToPath(import.meta.url)){
  const [inputArg,outArg]=process.argv.slice(2);if(!inputArg||!outArg)throw new Error('Usage: master-audio.mjs STEM_DIRECTORY ABSENT_OUTPUT_DIRECTORY');
  const input=path.resolve(inputArg),out=path.resolve(outArg);
  const manifest=JSON.parse(await readFile(path.join(input,'audio-manifest.json'),'utf8')),stems={};
  for(const name of ['music','sfx','voice'])stems[name]=readPcm(await readFile(path.join(input,`${name}.wav`)));
  const {tracks,stats}=masterMix(stems,manifest.voice.takes);
  await mkdir(out);
  for(const file of ['music.wav','sfx.wav','voice.wav',...manifest.voice.takes.map(x=>x.file)])await copyFile(path.join(input,file),path.join(out,file));
  for(const [name,samples]of Object.entries(tracks))await writeFile(path.join(out,`${name}.wav`),pcmWav(samples),{flag:'wx'});
  manifest.mix={ducking:'Music falls 60% around voice takes; attack 220 ms, release 360 ms',master:'Rebalanced original stems; PCM peak -1.5 dBFS; no LUFS conformance claim'};
  Object.assign(manifest.tracks,stats);
  await writeFile(path.join(out,'audio-manifest.json'),JSON.stringify(manifest,null,2),{flag:'wx'});
  console.log(`Mastered both mixes, retaining original stems: ${out}`);
}
