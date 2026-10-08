import React, {useLayoutEffect, useMemo} from 'react';
import {AbsoluteFill, Composition, Easing, Img, interpolate, registerRoot, staticFile, useCurrentFrame, useVideoConfig} from 'remotion';
import {Audio} from '@remotion/media';
import {ThreeCanvas} from '@remotion/three';
import {useLoader, useThree} from '@react-three/fiber';
import * as THREE from 'three';
import media from './film-media.json';

// One frame-driven camera travels through a character world. No autonomous animation loop.
// Gate, final wordmark and Clawd silhouette come from the canonical local assets.
const C={night:'#20221e',pale:'#eae5da',muted:'#91968a',orange:'#e0a485'};
const mono='Menlo, Consolas, monospace';
const clamp=(n:number)=>Math.max(0,Math.min(1,n));
const ease=Easing.bezier(.42,0,.28,1),glide=Easing.bezier(.16,1,.3,1);
const smooth=(p:number)=>p*p*(3-2*p);
function key(t:number,times:number[],values:number[],easing=ease){return interpolate(t,times,values,{extrapolateLeft:'clamp',extrapolateRight:'clamp',easing});}
function show(t:number,a:number,b:number){return key(t,[a,a+.36,b-.3,b],[0,1,1,0]);}
type V3=[number,number,number];
const cameraTimes=[0,1.75,4.6,7.7,9.6,12.5,13.6,16.6];
const cameraPoints:V3[]=[[-5.2,3.8,13],[0,2.6,7.6],[.2,1.8,-6.4],[5.2,3.7,-10.5],[4,3,-19],[-3.4,2.9,-22.5],[-2,3.7,-30],[2.6,2.5,-37.5]];
const lookPoints:V3[]=[[0,1.85,0],[0,1.65,-1],[0,1.1,-16],[0,1.5,-18],[0,1.5,-28],[0,1.7,-29],[0,1.3,-41],[0,1.5,-44]];
const cameraCurve=new THREE.CatmullRomCurve3(cameraPoints.map(p=>new THREE.Vector3(...p)),false,'catmullrom',.3);
const lookCurve=new THREE.CatmullRomCurve3(lookPoints.map(p=>new THREE.Vector3(...p)),false,'catmullrom',.3);
function cameraAt(t:number){
  // Monotone cubic timing keeps speed continuous at every scene boundary.
  const count=cameraTimes.length,step=1/(count-1),slopes=cameraTimes.slice(1).map((n,i)=>step/(n-cameraTimes[i]));
  const tangents=[slopes[0]*.35,...slopes.slice(1).map((s,i)=>2/(1/slopes[i]+1/s)),0];
  let u=1;
  if(t<=cameraTimes[0])u=0;
  else if(t<cameraTimes[count-1]){
    const i=cameraTimes.findIndex(n=>n>t)-1,dt=cameraTimes[i+1]-cameraTimes[i],p=(t-cameraTimes[i])/dt;
    u=(2*p*p*p-3*p*p+1)*i*step+(p*p*p-2*p*p+p)*dt*tangents[i]+(-2*p*p*p+3*p*p)*(i+1)*step+(p*p*p-p*p)*dt*tangents[i+1];
  }
  return {position:cameraCurve.getPoint(u),look:lookCurve.getPoint(u)};
}
function Camera({t}:{t:number}){
  const {camera}=useThree();
  useLayoutEffect(()=>{const p=cameraAt(t);camera.position.copy(p.position);camera.lookAt(p.look);camera.updateMatrixWorld();},[camera,t]);
  return null;
}
type Glyph={p:V3,size:number,glyph:number,alpha:number,warm?:number,phase?:number};
function atlas(){
  const canvas=document.createElement('canvas');canvas.width=1024;canvas.height=128;
  const ctx=canvas.getContext('2d')!;ctx.fillStyle='#fff';ctx.font='82px Menlo, monospace';ctx.textAlign='center';ctx.textBaseline='middle';
  ['·','+','~',':','^','/','|','_'].forEach((g,i)=>ctx.fillText(g,i*128+64,68));
  const texture=new THREE.CanvasTexture(canvas);texture.minFilter=THREE.LinearFilter;texture.magFilter=THREE.LinearFilter;return texture;
}
const vertex=`
attribute vec3 center;attribute float glyph;attribute float size;attribute float alpha;
attribute float warm;attribute float phase;
uniform float time;uniform float opacity;
varying vec2 texUV;varying float letter;varying float strength;varying float heat;
void main(){
 vec3 p=center;p.y+=sin(time*.7+phase)*.024;
 vec4 eye=modelViewMatrix*vec4(p,1.);
 float fog=1.-smoothstep(24.,76.,-eye.z);
 eye.xy+=position.xy*size;
 gl_Position=projectionMatrix*eye;
 texUV=uv;letter=glyph;strength=alpha*opacity*fog;heat=warm;
}`;
const fragment=`
uniform sampler2D letters;uniform vec3 ink;uniform vec3 accent;
varying vec2 texUV;varying float letter;varying float strength;varying float heat;
void main(){
 float a=texture2D(letters,vec2((floor(letter)+texUV.x)/8.,texUV.y)).a*strength;
 if(a<.009)discard;
 gl_FragColor=vec4(mix(ink,accent,heat),a);
 #include <tonemapping_fragment>
 #include <colorspace_fragment>
}`;
function Glyphs({points,t,opacity=1,depthWrite=false}:{points:Glyph[],t:number,opacity?:number,depthWrite?:boolean}){
  const {geometry,uniforms}=useMemo(()=>{
    const quad=new THREE.PlaneGeometry(1,1),g=new THREE.InstancedBufferGeometry();
    g.index=quad.index;g.attributes.position=quad.attributes.position;g.attributes.uv=quad.attributes.uv;
    const centers=new Float32Array(points.length*3),a:Record<string,Float32Array>={};
    for(const name of ['glyph','size','alpha','warm','phase'])a[name]=new Float32Array(points.length);
    points.forEach((p,i)=>{centers.set(p.p,i*3);a.glyph[i]=p.glyph;a.size[i]=p.size;a.alpha[i]=p.alpha;a.warm[i]=p.warm||0;a.phase[i]=p.phase||i*.31;});
    g.setAttribute('center',new THREE.InstancedBufferAttribute(centers,3));
    for(const [name,data]of Object.entries(a))g.setAttribute(name,new THREE.InstancedBufferAttribute(data,1));
    g.instanceCount=points.length;
    return {geometry:g,uniforms:{letters:{value:atlas()},time:{value:t},opacity:{value:opacity},ink:{value:new THREE.Color(C.pale)},accent:{value:new THREE.Color(C.orange)}}};
  },[points.length]);
  // Work streams change positions per frame without allocating another atlas or GPU geometry.
  if(geometry.userData.points!==points){
    points.forEach((p,i)=>{
      geometry.attributes.center.setXYZ(i,...p.p);geometry.attributes.alpha.setX(i,p.alpha);
    });
    geometry.attributes.center.needsUpdate=true;geometry.attributes.alpha.needsUpdate=true;
    geometry.userData.points=points;
  }
  uniforms.time.value=t;uniforms.opacity.value=opacity;
  return <mesh geometry={geometry} frustumCulled={false}><shaderMaterial vertexShader={vertex} fragmentShader={fragment} uniforms={uniforms} transparent depthWrite={depthWrite} side={THREE.DoubleSide}/></mesh>;
}
function landscape(){
  const points:Glyph[]=[];
  for(let z=8;z>-73;z-=.48)for(let x=-28;x<28;x+=.43){
    const water=Math.abs(x)<3.5;
    const y=water?-.24:.15+Math.pow(Math.max(0,Math.abs(x)-3.5),1.2)*.06+(Math.sin(x*.41+z*.07)+Math.cos(z*.22-x*.15))*.37;
    if(water&&(Math.round(z/.48)+Math.round(x/.43))%3!==0)continue;
    points.push({p:[x,y,z],glyph:water?2:Math.abs(x)>13?4:3,size:water?.21:.15,alpha:water?.22:.48,phase:x+z});
  }
  for(let i=0;i<60;i++){
    const side=i%2?1:-1,z=-4-(i*19%65),x=side*(5+(i*17%160)/10),h=1.7+(i*11%24)/10;
    for(let row=0;row<19;row++){
      const y=h*(1-row/19),span=.11+row*.095;
      for(let col=-span;col<=span;col+=.16)points.push({p:[x+col,.2+y,z+Math.sin(col*2)*.13],glyph:4,size:.17,alpha:.66});
      if(row>6)points.push({p:[x,.2+y,z],glyph:6,size:.16,alpha:.55});
    }
  }
  for(let i=0;i<125;i++)points.push({p:[Math.sin(i*2.399)*29,5+(i*23%62)/10,-8-(i*37%62)],glyph:i%5===0?1:0,size:i%5===0?.12:.075,alpha:.3,warm:i%23===0?.6:0});
  return points;
}
function ImagePlane({file,position,size,opacity=1,rotation=[0,0,0]}:{file:string,position:V3,size:[number,number],opacity?:number,rotation?:V3}){
  const texture=useLoader(THREE.TextureLoader,staticFile(file));texture.colorSpace=THREE.SRGBColorSpace;
  const actor=file==='clawd-crossing.svg';
  return <mesh position={position} rotation={rotation}><planeGeometry args={size}/><meshBasicMaterial map={texture} transparent opacity={opacity} alphaTest={actor?.05:0} depthWrite={actor} side={THREE.DoubleSide}/></mesh>;
}
function TextPlane({text,position,width,opacity=1,color=C.pale}:{text:string,position:V3,width:number,opacity?:number,color?:string}){
  const texture=useMemo(()=>{
    const canvas=document.createElement('canvas');canvas.width=1024;canvas.height=180;
    const ctx=canvas.getContext('2d')!;ctx.fillStyle=color;ctx.font='100px Menlo, monospace';ctx.textAlign='center';ctx.textBaseline='middle';ctx.fillText(text,512,90);
    const result=new THREE.CanvasTexture(canvas);result.colorSpace=THREE.SRGBColorSpace;return result;
  },[text,color]);
  return <mesh position={position}><planeGeometry args={[width,width*180/1024]}/><meshBasicMaterial map={texture} transparent opacity={opacity} depthWrite={false} side={THREE.DoubleSide}/></mesh>;
}
function Package({t,z=-18}:{t:number,z?:number}){
  const fold=key(t,[5.9,7.5],[0,1]);
  return <group position={[0,1.65,z]} rotation={[.08,key(t,[5,9.3],[-.23,.35]),.015*Math.sin(t)]}>
    <mesh><boxGeometry args={[2.25,1.5,1.6]}/><meshBasicMaterial color={C.night} transparent opacity={.94}/></mesh>
    <lineSegments><edgesGeometry args={[new THREE.BoxGeometry(2.25,1.5,1.6)]}/><lineBasicMaterial color={C.pale} transparent opacity={.56}/></lineSegments>
    <mesh position={[0,.75+key(t,[6.1,7.5],[1,0]),0]} rotation={[key(t,[6.1,7.5],[-.8,0]),0,0]}>
      <boxGeometry args={[2.3,.075,1.65]}/><meshBasicMaterial color={C.orange} transparent opacity={fold*.85}/>
    </mesh>
    <TextPlane text="lintel.work/1" position={[0,-.01,.805]} width={2.08} opacity={key(t,[6.8,7.6],[0,.8])}/>
  </group>;
}
function Work({t}:{t:number}){
  const points=useMemo(()=>{
    const p:Glyph[]=[];
    for(let i=0;i<420;i++){
      const strand=i%3,j=Math.floor(i/3),u=j/140;
      const progress=clamp((t-(5+strand*.42)-u*.7)/2.25),q=smooth(progress),sx=-5.4+u*2.5,sy=2.7-strand*1.05;
      p.push({p:[sx*(1-q)+Math.sin(q*Math.PI)*.8,sy*(1-q)+1.8*q+Math.sin(q*Math.PI)*.7,-16+q*(-2.1)-u*.2],glyph:i%8,size:.085,alpha:Math.sin(progress*Math.PI)*.82,warm:i%11===0?.8:0});
    }
    return p;
  },[t]);
  return <group>
    <Glyphs points={points} t={t}/>
    {['CLAUDE.md','MEMORY.md','session.jsonl'].map((name,i)=><TextPlane key={name} text={name} position={[-4,2.7-i*1.05,-16]} width={3.5} opacity={show(t,4.65+i*.18,7.35+i*.2)}/>) }
    <Package t={t}/>
  </group>;
}
function Review({t}:{t:number}){
  return <group position={[0,0,-29]}>
    {['Preview','Approve','Verify'].map((label,i)=>{
      const activation=10.2+i*.95,on=key(t,[activation-.2,activation,activation+.4,activation+.9],[.32,1,1,.58]);
      return <group key={label} position={[(i-1)*3.8,1.8,0]}>
        <mesh><planeGeometry args={[3.1,2.7]}/><meshBasicMaterial color={C.night} transparent opacity={.94}/></mesh>
        <lineSegments><edgesGeometry args={[new THREE.PlaneGeometry(3.1,2.7)]}/><lineBasicMaterial color={C.pale} transparent opacity={on*.28}/></lineSegments>
        <TextPlane text={label} position={[0,.7,.03]} width={2.9} opacity={on}/>
        {Array.from({length:6},(_,j)=><mesh key={j} position={[-.15+(j%2)*.1,.18-j*.22,.04]}><planeGeometry args={[j%3===0?1.9:1.1,.018]}/><meshBasicMaterial color={C.pale} transparent opacity={on*.28}/></mesh>)}
      </group>;
    })}
  </group>;
}
function Bridge({t}:{t:number}){
  const points=useMemo(()=>{
    const p:Glyph[]=[];
    for(let x=-5;x<5.1;x+=.17){
      const y=.48+.58*Math.sin((x+5)/10*Math.PI);
      for(let z=-44.8;z<-42.3;z+=.18)p.push({p:[x,y,z],glyph:7,size:.16,alpha:.78});
      for(const z of [-44.8,-42.3])for(let d=0;d<2;d++)p.push({p:[x,y+.18+d*.23,z],glyph:7,size:.15,alpha:z===-42.3?.55:.7});
    }
    for(let i=0;i<14;i++){
      const x=-5+i*10/13,y=.48+.58*Math.sin((x+5)/10*Math.PI);
      for(const z of [-44.8,-42.3])for(let j=0;j<5;j++)p.push({p:[x,y+j*.1,z],glyph:6,size:.16,alpha:z===-42.3?.6:.76});
    }
    return p;
  },[]);
  const x=key(t,[13.6,16.5],[-2.7,2.0],p=>p),deck=.48+.58*Math.sin((x+5)/10*Math.PI),cam=cameraAt(t);
  const gait=(x+2.7)*6.4,lift=Math.abs(Math.sin(gait))*.022;
  const scaleX=1-(1-lift/.022)*.025,scaleY=1-(1-lift/.022)*.022;
  // The foot follows the same deck function as the bridge; a small whole-body lift carries the cargo.
  const y=deck+.75*scaleY/2+lift;
  return <group>
    <Glyphs points={points} t={t} depthWrite/>
    <group position={[x,y,-43.5]} rotation={[0,Math.atan2(cam.position.x-x,cam.position.z+43.5),Math.sin(gait)*.007]} scale={[scaleX,scaleY,1]}>
      <ImagePlane file="clawd-crossing.svg" position={[0,0,0]} size={[1.2,.75]}/>
      <mesh position={[.1,.53,-.035]}><boxGeometry args={[.44,.22,.19]}/><meshBasicMaterial color={C.pale}/></mesh>
      <mesh position={[.1,.53,.065]}><planeGeometry args={[.028,.22]}/><meshBasicMaterial color={C.orange}/></mesh>
    </group>
    <TextPlane text="carry it forward" position={[0,3,-45]} width={6} opacity={show(t,13.7,16.65)}/>
    <Ripple t={t} at={14.4} position={[0,-.2,-41]}/>
  </group>;
}
function Ripple({t,at,position}:{t:number,at:number,position:V3}){
  return <group position={position} rotation={[-Math.PI/2,0,0]}>
    {[0,1,2].map(i=>{
      const age=clamp((t-at-i*.28)/2.6),r=.15+age*3.8;
      return <mesh key={i}><ringGeometry args={[r,r+.009,100]}/><meshBasicMaterial color={C.pale} transparent opacity={age>0&&age<1?Math.pow(1-age,2)*.3:0} depthWrite={false} side={THREE.DoubleSide}/></mesh>;
    })}
  </group>;
}
const cursorTimes=[0,.55,1.8,2.4,4.6,7.5,9.6,10.2,10.65,11.15,11.6,12.1,12.6,13.5,15.7,16.6];
const cursorPoints:V3[]=[[2.45,.27,.08],[2.45,.27,.08],[.1,1.2,-.1],[0,1.1,-3],[-4,1,-15],[0,.76,-17.1],[-3.8,1,-28.9],[-3.8,1,-28.9],[-3.8,1,-28.9],[0,1,-28.9],[0,1,-28.9],[3.8,1,-28.9],[3.8,1,-28.9],[-1.8,1.32,-42.5],[1.6,1.32,-43.5],[3,1.32,-43.5]];
function cursorAt(t:number):V3{return [0,1,2].map(a=>key(t,cursorTimes,cursorPoints.map(p=>p[a]))) as V3;}
function Cursor({t}:{t:number}){
  return <mesh position={cursorAt(t)} renderOrder={6}>
    <planeGeometry args={[key(t,[0,2,4.8,7.5,10.2,13.5],[.85,.42,.5,2.25,1.72,.64]),key(t,[0,2,5],[.38,.1,.065])]}/>
    <meshBasicMaterial color={C.orange} transparent opacity={key(t,[0,.5,16.55,16.65],[0,1,1,0])} depthWrite={false} side={THREE.DoubleSide}/>
  </mesh>;
}
function World({t}:{t:number}){
  const points=useMemo(landscape,[]);
  return <>
    <Camera t={t}/><ImagePlane file="night-horizon.svg" position={[0,28,-110]} size={[230,230*660/1774]} opacity={.66}/>
    <Glyphs points={points} t={t} opacity={key(t,[0,1.5],[.5,1])}/>
    <ImagePlane file="mark-night.svg" position={[0,1.8,0]} size={[6,6*430/720]} opacity={key(t,[0,.7],[0,1])}/>
    <Ripple t={t} at={2.4} position={[0,-.2,-2]}/><Work t={t}/><Review t={t}/><Bridge t={t}/><Cursor t={t}/>
  </>;
}
function KineticCopy({t,at,end,word,sub}:{t:number,at:number,end:number,word:string,sub:string}){
  const disappear=key(t,[end-.55,end],[0,1]);
  return <div style={{position:'absolute',left:126,top:106,color:C.pale,opacity:1-disappear}}>
    <div style={{fontFamily:'Baskerville, Georgia, serif',fontSize:154,lineHeight:1.08,letterSpacing:-5,display:'flex',overflow:'hidden'}}>
      {Array.from(word).map((c,i)=>{const p=key(t,[at+i*.035,at+.85+i*.035],[0,1],glide);return <span key={i} style={{display:'block',transform:`translateY(${(1-p)*160}px) rotate(${(1-p)*6}deg)`,opacity:p}}>{c}</span>;})}
    </div>
    <div style={{fontFamily:mono,fontSize:23,letterSpacing:-.7,marginTop:24,opacity:key(t,[at+.48,at+1],[0,.7])}}>{sub}</div>
  </div>;
}
function Closing({t}:{t:number}){
  const s=key(t,[16.75,18.1],[1.13,1],glide),width=1000,baseX=460,baseY=338,foot=media.word.foot;
  const cam=new THREE.PerspectiveCamera(52,1920/1080,.1,110),v=cameraAt(16.6);
  cam.position.copy(v.position);cam.lookAt(v.look);cam.updateMatrixWorld();
  const projected=new THREE.Vector3(...cursorAt(16.6)).project(cam),startX=(projected.x+1)*960,startY=(1-projected.y)*540;
  const x=baseX+foot.x*width/media.word.width,y=baseY+foot.y*width/media.word.width,p=key(t,[16.6,17.9],[0,1]);
  return <AbsoluteFill style={{opacity:key(t,[16.55,16.8],[0,1])}}>
    <AbsoluteFill style={{background:C.night,opacity:key(t,[16.6,17.45],[0,1])}}/>
    <Img src={staticFile('word-night.svg')} style={{position:'absolute',left:baseX,top:baseY,width,scale:s,opacity:key(t,[16.8,17.9],[0,1]),transformOrigin:'100% 100%'}}/>
    <div style={{position:'absolute',left:startX+(x-startX)*p,top:startY+(y-startY)*p,width:38+(foot.width*width/media.word.width-38)*p,height:4+(foot.height*width/media.word.width-4)*p,background:C.orange,borderRadius:4.75,opacity:key(t,[16.55,16.65],[0,1])}}/>
    <div style={{position:'absolute',left:0,right:0,top:713,textAlign:'center',fontFamily:mono,fontSize:25,letterSpacing:-1,color:C.pale,opacity:key(t,[17.5,18.35],[0,.82])}}>Your work. Your call.</div>
    <div style={{position:'absolute',left:0,right:0,top:778,textAlign:'center',fontFamily:'system-ui, sans-serif',fontSize:22,color:C.muted,opacity:key(t,[18.1,18.7],[0,1])}}>A clean, personal tool for Claude Code users.</div>
    <div style={{position:'absolute',left:0,right:0,bottom:75,textAlign:'center',fontFamily:mono,fontSize:14,letterSpacing:3,color:C.orange,opacity:key(t,[18.45,19.05],[0,.75])}}>PREVIEW · LINTEL.PAGE</div>
  </AbsoluteFill>;
}
const Intro=({voiceover=false}:{voiceover?:boolean})=>{
  const frame=useCurrentFrame(),{fps}=useVideoConfig(),t=frame/fps;
  return <AbsoluteFill style={{background:C.night,color:C.pale,overflow:'hidden'}}>
    <ThreeCanvas width={1920} height={1080} camera={{fov:52,near:.1,far:180}} gl={{alpha:true,antialias:true}} flat><World t={t}/></ThreeCanvas>
    <KineticCopy t={t} at={4.85} end={8.6} word="Keep." sub="instructions · memory · conversations"/>
    <KineticCopy t={t} at={9.8} end={13.3} word="Know." sub="preview → approve → verify"/>
    <Closing t={t}/>
    <AbsoluteFill style={{pointerEvents:'none',boxShadow:'inset 0 0 170px 25px #11140f3a'}}/>
    {media.audio?<Audio src={staticFile(voiceover?'mix-voice.wav':'mix.wav')} premountFor={fps}/>:null}
  </AbsoluteFill>;
};
const Native=()=>{
  const f=useCurrentFrame(),{fps}=useVideoConfig();let elapsed=0;let shot=media.native.at(-1),index=media.native.length-1;
  for(let i=0;i<media.native.length;i++){if(f<(elapsed+media.native[i].seconds)*fps){shot=media.native[i];index=i;break;}elapsed+=media.native[i].seconds;}
  return <AbsoluteFill style={{background:'#faf9f5',color:'#3d3d3a'}}>
    <Img src={staticFile(shot.file)} style={{position:'absolute',top:8,left:0,width:1600,height:900,objectFit:'contain'}}/>
    <div style={{position:'absolute',left:80,right:80,top:920,borderTop:'1px solid #c16a4740',paddingTop:13,fontFamily:mono,fontSize:15,color:'#80796e'}}>REAL macOS APP · SYNTHETIC DATA · EDITED</div>
    <div style={{position:'absolute',left:80,top:953,fontFamily:'system-ui, sans-serif',fontSize:25}}>{String(index+1).padStart(2,'0')} / {shot.caption}</div>
  </AbsoluteFill>;
};
const Root=()=> <>
  <Composition id="LintelIntro" component={Intro} width={1920} height={1080} fps={60} durationInFrames={1200} defaultProps={{voiceover:false}}/>
  <Composition id="LintelIntroVoice" component={Intro} width={1920} height={1080} fps={60} durationInFrames={1200} defaultProps={{voiceover:true}}/>
  {media.native.length>0?<Composition id="LintelNative" component={Native} width={1600} height={1000} fps={30} durationInFrames={900}/>:null}
</>;
registerRoot(Root);
