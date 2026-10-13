import test from 'node:test';
import assert from 'node:assert/strict';
import { newRun, jump, pressJump, releaseJump, cancelJump, holdPower, advance, clawdPose, spawnStar, points, BODY, clawdBody, OBSTACLE_KINDS, OBSTACLE_BOUNDS, spawnObstacle, spawnInterval, isHard, PLACE_LENGTH, PLACES, scene, drawScenery } from '../apps/site/clawd-game.mjs';

// Current speed tuning used by independent reachability checks.
const PX = 48, GROUND = 188, HEIGHT = 40, HIT_Y = 24;
const MIN_SPEED = 260, MAX_SPEED = 350;
// Ordinary reward admission includes the bounded rows and obstacle-specific upward adjustment.
const STAR_JITTER = [0, 5, -4, 3, -2, 6, -5, 4, -3, 2];
const HIGH_HEIGHTS = [...new Set(STAR_JITTER.map(j=>56+Math.max(-4,Math.min(4,j))))];
const STAR_HEIGHTS = Array.from({length:23},(_,i)=>95+i);
// A seeded generator so the obstacle spawn sequence is fixed, not left to chance.
const seeded = seed => () => { seed |= 0; seed = seed + 0x6d2b79f5 | 0; let t = Math.imul(seed ^ seed >>> 15, 1 | seed); t = t + Math.imul(t ^ t >>> 7, 61 | t) ^ t; return ((t ^ t >>> 14) >>> 0) / 4294967296; };

test('a first-frame jump lifts Clawd, rejects midair jumps, and lands again', () => {
  const run = newRun(); run.status = 'running'; jump(run);
  advance(run, 0, 1000);
  advance(run, 1 / 60, 1000);
  assert.ok(run.y > 0, 'the initial zero-time animation frame must not cancel the jump');
  const velocity = run.velocity; jump(run); assert.equal(run.velocity, velocity);
  for (let i = 0; i < 60; i++) advance(run, 1 / 60, 1000, () => .5);
  assert.equal(run.y, 0);
});

test('the same obstacle ends a grounded run and can be cleared by a jump', () => {
  const grounded = newRun(); grounded.status = 'running'; grounded.obstacles = [{x:80,width:22,height:25}];
  advance(grounded, 1 / 60, 1000); assert.equal(grounded.status, 'over');
  const airborne = newRun(); airborne.status = 'running'; jump(airborne);
  for (let i = 0; i < 10; i++) advance(airborne, 1 / 60, 1000);
  airborne.obstacles = [{x:80,width:22,height:25}];
  advance(airborne, 1 / 60, 1000); assert.equal(airborne.status, 'running');
});

test('ready, paused and ended runs stay still; a fresh run clears previous progress', () => {
  for (const status of ['ready','paused','over']) {
    const run = newRun(); run.status = status; run.distance = 100;
    const previous = structuredClone(run); advance(run, 1, 390); jump(run);
    assert.deepEqual(run, previous);
  }
  assert.equal(newRun().distance, 0); assert.deepEqual(newRun().obstacles, []);
});

test('the sprite is one connected body with four continuous legs and no detached feet drawing', () => {
  // Verified by geometry, not by eye: the canonical path is a single connected subpath, exactly the
  // four short legs hang from the shared body, and no separate foot geometry is drawn or stored.
  assert.equal((BODY.match(/M/g) ?? []).length, 1, 'one M: the shape is connected, not four loose parts');
  assert.equal((BODY.match(/V50/g) ?? []).length, 4, 'four short legs descend from the shared body');
  assert.match(BODY, /V50H60V40H56V50/, 'the legs are straight H/V runs, not animated curves');
  assert.doesNotMatch(BODY, /foot|feet/i);
});

test('the standing pose is the canonical shape; only running, airborne and landing move it', () => {
  const canonical = { anchor: 0, scaleX: 1, scaleY: 1, legs: [14,24,50,60].map(x => ({x, bottom:50})), eyes: [[20,10,4,10],[56,10,4,10]] };
  for (const status of ['ready', 'paused']) {
    const run = newRun(); run.status = status; run.distance = 100;
    assert.deepEqual(clawdPose(run), canonical, `${status} has no standing scale`);
  }
  const run = newRun(); run.status = 'running';
  for (let i = 0; i < 8; i++) advance(run, 1 / 60, 1000);
  const stepping = clawdPose(run);
  assert.equal(stepping.anchor, 0, 'grounded running keeps the feet anchored on the ground');
  jump(run); advance(run, 1 / 60, 1000, () => 0);
  const air = clawdPose(run);
  assert.equal(air.anchor, 0, 'the airborne anchor is the same anchored ground goal');
  assert.ok(air.scaleY > 1, 'the rising body stretches vertically instead of tucking legs');
  assert.ok(air.scaleY > stepping.scaleY, 'the ascent stretches taller than the grounded stride');
  assert.deepEqual(clawdPose(run, true), canonical, 'reduced motion holds the canonical shape static');
  // Pausing mid-air or just after landing freezes the exact canonical shape, matching the comment.
  const airborne = newRun(); airborne.status = 'paused'; airborne.y = 40; airborne.velocity = 200;
  assert.deepEqual(clawdPose(airborne), canonical, 'a paused mid-air jump holds the canonical shape');
  const justLanded = newRun(); justLanded.status = 'paused'; justLanded.landing = .14;
  assert.deepEqual(clawdPose(justLanded), canonical, 'a paused just-landed body holds the canonical shape');
});

test('successive stars take visibly different heights that a real jump can reach', () => {
  // The height function is deterministic, so the sequence can be checked directly.
  const sequence = [];
  for (let i = 0; i < 12; i++) sequence.push(spawnStar({ x: 0 }, i).y);
  assert.ok(new Set(sequence).size >= 9, `heights must vary across spawns, saw ${new Set(sequence).size}`);
  assert.ok(Math.max(...sequence) - Math.min(...sequence) >= 40, `the spread is at least 40px, saw ${Math.max(...sequence) - Math.min(...sequence)}`);
  for (const y of sequence) assert.ok([...STAR_HEIGHTS,...HIGH_HEIGHTS].includes(y), `${y} is a normal or held star height`);
  // Three consecutive stars sit on three different rows, so the change is visible in a play session.
  assert.equal(spawnStar({ x: 0 }, 0).y, 98);
  assert.equal(spawnStar({ x: 0 }, 1).y, 109);
  assert.equal(spawnStar({ x: 0 }, 2).y, 111);
  // And `advance` really threads those heights through a live run: one generator, a wide board so
  // several obstacles spawn, and at least three actually spawned stars with a visible height spread.
  const run = newRun(); run.status = 'running'; const random = seeded(3);
  const spawned = [];
  for (let i = 0; i < 1200; i++) {
    const before = run.spawns;
    advance(run, 1 / 60, 4000, random);
    if (run.spawns !== before) spawned.push(run.stars.at(-1)?.y);
  }
  assert.ok(spawned.length >= 3, `advance must really spawn several stars, saw ${spawned.length}`);
  assert.ok(spawned.every(y => [...STAR_HEIGHTS,...HIGH_HEIGHTS].includes(y)), 'every advance-spawned star uses the normal/held height scheme');
  assert.ok(Math.max(...spawned) - Math.min(...spawned) >= 40, 'the advance-spawned heights visibly spread');
});

test('nobody can collect a star from the ground: every height needs a real jump', () => {
  for (const y of STAR_HEIGHTS) {
    const reach = Math.abs(y - (GROUND - HEIGHT / 2)); // grounded body centre
    assert.ok(reach >= HIT_Y, `star at ${y} is out of grounded reach (need a jump)`);
  }
});

// Fly one real jump over a single obstacle. `launchX` is the obstacle x on the frame the jump
// starts; the caller scans every launch x, so no timing constant is assumed. The trial counts only
// when the star was collected exactly once, that original obstacle was passed, Clawd landed, the
// run is still running, no unrelated obstacle/star ever appeared, and the requested speed held.
function fly(starY, obstacleH, obstacleW, speed, launchX, trace, power=0) {
  const run = newRun(); run.status = 'running';
  // advance() recomputes speed as min(350, 260 + distance/300); seed distance so the requested speed
  // holds from the first step. At 350 (the clamp) any distance past 27000 pins it exactly at 350;
  // at 260 the game keeps accelerating from here, which is the real behaviour, not a downgrade.
  run.speed = speed; run.distance = (speed - 260) * 300;
  const obstacle = { x: 600, width: obstacleW, height: obstacleH, kind: 'stone', passed: false };
  run.obstacles = [obstacle];
  run.stars = [{ x: 612, y: starY }];
  run.spawn = 1e6; // a finite spawn delay far beyond this short trial: no unrelated star can appear
  let launched = false, landed = false, speedHeld = run.speed === speed, minSpeed = run.speed, maxSpeed = run.speed;
  for (let step = 0; step < 200000; step++) {
    if (!launched && obstacle.x <= launchX) { if (power) pressJump(run); else jump(run); launched = true; }
    if (run.holding && holdPower(run) === 1) releaseJump(run);
    advance(run, 1 / 240, 0, () => 0);
    if (run.obstacles.length > 1 || run.stars.length > 1) return false; // spawn delay holds: no extra spawns
    if (speed === 350 && run.speed !== 350) speedHeld = false; // the max-speed trial must never drift
    minSpeed = Math.min(minSpeed, run.speed); maxSpeed = Math.max(maxSpeed, run.speed);
    if (launched && run.y === 0 && run.velocity === 0) landed = true;
    if (run.status === 'over' || (obstacle.passed && landed)) break;
  }
  if (trace) { trace.minSpeed = minSpeed; trace.maxSpeed = maxSpeed; }
  return speedHeld && run.collected === 1 && obstacle.passed && landed && run.status === 'running';
}
const cleanlyCollectible = (starY, obstacleH, obstacleW, speed) => {
  for (let launchX = 40; launchX <= 500; launchX++) if (fly(starY, obstacleH, obstacleW, speed, launchX)) return true;
  return false;
};

test('a legal jump collects every star height at normal and maximum speed, tallest obstacle included', () => {
  let cases = 0;
  for (const speed of [MIN_SPEED, MAX_SPEED]) {
    for (const height of [18, 29]) { // shortest and tallest obstacle
      for (const width of [20, 28]) {
        for (const starY of STAR_HEIGHTS) {
          cases++;
          assert.ok(cleanlyCollectible(starY, height, width, speed),
            `star ${starY}px over a ${height}x${width} obstacle at ${speed}px/s is collectible with a clean jump`);
        }
      }
    }
  }
  assert.equal(cases, 2 * 2 * 2 * STAR_HEIGHTS.length, 'every advertised speed/height/width/star case ran');
  // Prove the cases really ran at their advertised speed. At the 350 clamp speed can never fall back
  // toward 260; at 260 the run starts at 260 and only accelerates upward, which is the real game.
  const fast = {};
  assert.ok(fly(STAR_HEIGHTS[0], 29, 28, MAX_SPEED, 118, fast), 'a 350px/s run collects, clears and lands cleanly');
  assert.equal(fast.minSpeed, MAX_SPEED, 'the maximum-speed trial never drifted below 350');
  assert.equal(fast.maxSpeed, MAX_SPEED, 'the maximum-speed trial stayed pinned at the 350 clamp');
  const slow = {};
  assert.ok(fly(STAR_HEIGHTS[0], 29, 28, MIN_SPEED, 118, slow), 'a 260px/s run collects, clears and lands cleanly');
  assert.equal(slow.minSpeed, MIN_SPEED, 'the normal-speed trial starts at 260, not a hidden 350');
});

test('timed jumps collect stars, clear obstacles and score each star only once', () => {
  const run = newRun(); run.status = 'running'; const random = seeded(11);
  for (let i = 0; i < 2400 && run.collected < 3; i++) {
    const obstacle = run.obstacles.find(item => item.x + item.width >= PX);
    if (obstacle && obstacle.x < 135 && run.y === 0) jump(run);
    advance(run, 1 / 60, 720, random);
    assert.equal(run.status, 'running', 'a correctly timed jump must clear the obstacle');
  }
  assert.ok(run.collected >= 3, 'three stars are collected on a steady jump rhythm');
  assert.equal(points(run), Math.floor(run.distance / 10) + run.collected * 25);
  assert.ok(run.cleared >= 2);
  const caught = run.collected; advance(run, 1 / 60, 720, random); assert.equal(run.collected, caught);
});

test('a press just before landing buffers one jump and restart clears stars and particles', () => {
  const run=newRun();run.status='running';run.y=2;run.velocity=-300;
  jump(run);advance(run,1/60,720);
  assert.ok(run.velocity>0,'late input is consumed at landing');
  assert.equal(run.buffer,0);
  const fresh=newRun();assert.equal(fresh.collected,0);assert.deepEqual(fresh.particles,[]);
});

test('jump height stays consistent at 30 and 144 frames per second', () => {
  const heights=[];
  for(const fps of [30,144]){
    const run=newRun();run.status='running';jump(run);
    let height=0;
    for(let i=0;i<fps;i++){advance(run,1/fps,1200);height=Math.max(height,run.y);}
    heights.push(height);assert.equal(run.y,0);
  }
  assert.ok(Math.abs(heights[0]-heights[1])<1);
});

// Fly a real ordinary run at a given frame rate and a requested speed, no input, and report the
// grounded pose bounds plus whether the ballistics/scoring still behave. `advance` is the only thing
// driving the game, so these are outcome checks over real gameplay rather than pose formulas.
function flyRun({ fps, speed, seconds = 20 }) {
  const run = newRun(); run.status = 'running';
  run.speed = speed; run.distance = (speed - 260) * 300; // hold the requested speed from the first step
  const wide = 100000; // no obstacle can reach Clawd; this isolates ordinary grounded running
  let footPeak = 0, footFloor = 0;
  for (let i = 0; i < fps * seconds; i++) {
    advance(run, 1 / fps, wide, () => .5);
    if (run.y > 0 || run.velocity > 0) continue; // only ordinary grounded running
    const pose = clawdPose(run);
    footPeak = Math.max(footPeak, pose.anchor);
    footFloor = Math.min(footFloor, pose.anchor);
  }
  return { run, footPeak, footFloor };
}

test('ordinary running anchors the feet on the ground and never bobs the body', () => {
  // The repeated ground jolt came from a whole-body lift keyed to travelled distance. The repair
  // keeps grounded feet exactly on GROUND: over a real run the pose anchor must hold at 0, never
  // lifting or dropping the body, so no vertical ground beat can exist to jolt it.
  for (const speed of [260, 280, 350]) {
    for (const fps of [30, 60, 144]) {
      const { run, footPeak, footFloor } = flyRun({ fps, speed });
      assert.ok(run.distance > 0, `the ${fps}fps ${speed}px/s run really travelled`);
      assert.equal(footPeak, 0, `${fps}fps ${speed}px/s never lifts the grounded body`);
      assert.equal(footFloor, 0, `${fps}fps ${speed}px/s never drops the grounded body`);
    }
  }
});

test('short straight legs alternate while the chest stays level across frame rates', () => {
  for (const speed of [260, 280, 350]) for (const fps of [30, 60, 144]) {
    const run = newRun(); run.status = 'running'; run.distance = (speed - 260) * 300; run.speed = speed;
    const tips = [], positions = [];
    for (let i = 0; i < fps * 2; i++) {
      advance(run, 1 / fps, 100000, () => .5);
      const pose = clawdPose(run);
      assert.equal(pose.scaleX, 1, 'ordinary steps never pulse the whole body');
      assert.equal(pose.scaleY, 1, 'the chest has a stable height');
      assert.equal(pose.legs.length, 4);
      assert.ok(pose.legs.filter(l => l.bottom === 50).length >= 2, 'one pair supports each step');
      assert.ok(pose.legs.every(l => l.bottom >= 46 && l.bottom <= 50), 'legs stay short');
      assert.equal(pose.legs[0].bottom, pose.legs[3].bottom, 'outer legs share a step');
      assert.equal(pose.legs[1].bottom, pose.legs[2].bottom, 'inner legs share the opposite step');
      for (let j = 1; j < 4; j++) assert.ok(pose.legs[j].x > pose.legs[j-1].x + 6, 'legs remain distinct');
      const shape = clawdBody(pose.legs);
      assert.equal((shape.match(/M/g) ?? []).length, 1, 'moving legs remain in the connected silhouette');
      assert.doesNotMatch(shape, /[LCQSA]/, 'all edges stay straight and orthogonal');
      tips.push(pose.legs[0].bottom); positions.push(pose.legs[0].x);
    }
    assert.ok(Math.max(...tips) - Math.min(...tips) > 3, 'legs visibly lift, rather than slide motionless');
    assert.ok(Math.max(...positions) - Math.min(...positions) > 2, 'a stride includes forward/back travel');
    let lifts = 0;
    for (let i = 1; i < tips.length; i++) if (tips[i] < 49.9 && tips[i-1] >= 49.9) lifts++;
    assert.ok(lifts >= 3 && lifts <= 8, `the ${fps}fps ${speed}px/s gait has a calm cadence (${lifts} steps)`);
  }
  assert.equal(clawdBody(clawdPose(newRun()).legs), BODY, 'standing and film keep the canonical silhouette');
});

test('ordinary grounded running arms no landing squash; only a real landing settles, and it is finite', () => {
  const ground = flyRun({ fps: 60, speed: 280 });
  assert.equal(ground.run.landing, 0, 'ordinary running never arms a landing squash');
  const landed = newRun(); landed.status = 'running';
  jump(landed);
  // Advance until it actually touches down, using the physics' own single-frame granularity.
  let landedPose = null;
  for (let i = 0; i < 480; i++) {
    const wasAirborne = landed.y > 0 || landed.velocity > 0;
    advance(landed, 1 / 240, 1200, () => .5);
    if (wasAirborne && landed.y === 0) { landedPose = clawdPose(landed); break; }
  }
  assert.equal(landed.y, 0, 'the airborne body really landed');
  assert.ok(landedPose.scaleX > landedPose.scaleY, 'a real landing squashes the body wider than it is tall');
  for (let i = 0; i < 60; i++) advance(landed, 1 / 60, 1200, () => .5);
  assert.equal(landed.landing, 0, 'the landing settle runs out within its finite window');
  // The settled body returns to anchored feet; no leftover lift or repeated bounce remains.
  assert.equal(clawdPose(landed).anchor, 0, 'the settled body is anchored again');
});

test('a real jump keeps the accepted flight stretch and scoring', () => {
  // The grounded carrying motion changed; the jump itself must not. A rising body still stretches
  // vertically, and the points formula is untouched.
  const run = newRun(); run.status = 'running'; jump(run);
  let sawStretch = false;
  for (let i = 0; i < 60; i++) {
    advance(run, 1 / 60, 1200);
    if (run.velocity > 0 && clawdPose(run).scaleY > 1) sawStretch = true;
  }
  assert.ok(sawStretch, 'the rising body stretches vertically while airborne');
  assert.equal(clawdPose(run).anchor, 0, 'the landed body is anchored on the ground again');
  assert.equal(points({ distance: 1234, collected: 5 }), Math.floor(1234 / 10) + 5 * 25, 'scoring formula is unchanged');
});


test('press takes off immediately; holding raises the same jump, caps and never auto-repeats',()=>{
  const peaks=[];
  for(const duration of [.05,.17,.5,1.2]){
    const run=newRun();run.status='running';run.spawn=1e6;
    pressJump(run);assert.equal(run.velocity,480,'the down event starts the original jump');
    let peak=0;
    for(let t=0;t<duration;t+=1/240){advance(run,1/240,2000);peak=Math.max(peak,run.y);}
    assert.ok(holdPower(run)<=1);
    const before={y:run.y,velocity:run.velocity};releaseJump(run);
    assert.equal(run.holding,false);assert.equal(run.holdTime,0);
    assert.deepEqual({y:run.y,velocity:run.velocity},before,'release adds no second jump or impulse');
    for(let i=0;i<360;i++){advance(run,1/240,2000);peak=Math.max(peak,run.y);}
    assert.equal(run.y,0);assert.equal(run.velocity,0);peaks.push(peak);
  }
  assert.ok(Math.abs(peaks[0]-480**2/(2*1450))<.1,'short taps retain the original ballistic height');
  assert.ok(peaks[1]>peaks[0]+15);assert.ok(peaks[2]>peaks[1]+15);
  assert.ok(Math.abs(peaks[3]-peaks[2])<.1,'holding past the finite window cannot add height or repeat');
  assert.ok(peaks[2]<150,'the held leap stays inside the board');
});

test('held height is consistent at 30/60/144fps; cancellation and midair re-press cannot relaunch',()=>{
  const peaks=[];
  for(const fps of [30,60,144]){
    const run=newRun();run.status='running';run.spawn=1e6;pressJump(run);
    let peak=0;
    for(let i=0;i<fps*2;i++){advance(run,1/fps,2000);peak=Math.max(peak,run.y);}
    assert.equal(run.y,0,'a held key does not auto-repeat on landing');
    releaseJump(run);assert.equal(run.velocity,0);peaks.push(peak);
    pressJump(run);advance(run,.1,2000);cancelJump(run);
    const velocity=run.velocity;releaseJump(run);assert.equal(run.velocity,velocity,'cancelled release adds no impulse');
    assert.equal(run.holdTime,0);assert.equal(run.buffer,0);
    for(let i=0;i<fps;i++)advance(run,1/fps,2000);
    assert.equal(run.y,0);assert.equal(run.velocity,0);
  }
  assert.ok(Math.max(...peaks)-Math.min(...peaks)<1);
  const paused=newRun();paused.status='paused';pressJump(paused);releaseJump(paused);
  assert.equal(paused.holding,false);assert.equal(paused.velocity,0);
  const midair=newRun();midair.status='running';midair.spawn=1e6;
  pressJump(midair);advance(midair,.1,2000);releaseJump(midair);
  const speed=midair.velocity;pressJump(midair);
  assert.equal(midair.velocity,speed,'another down event in the air cannot add an impulse');
  assert.equal(midair.boosting,false,'re-pressing cannot renew the lift window in midair');
});

test('a new run starts briskly and accelerates continuously without exceeding the speed cap',()=>{
  const run=newRun();run.status='running';run.spawn=1e6;
  assert.equal(run.speed,260);
  let previous=run.speed;
  for(let i=0;i<60*110;i++){
    advance(run,1/60,2000);
    assert.ok(run.speed>=previous&&run.speed-previous<.1,'acceleration has no step change');
    assert.ok(run.speed<=350);previous=run.speed;
  }
  assert.equal(run.speed,350);
});

test('high double stars require a held ascent, remain cleanly reachable at every speed, and award once',()=>{
  for(let i=0;i<20;i++)assert.equal(spawnStar({x:100},i).value===50,i%4===3,'one high reward in each group of four');
  for(const y of HIGH_HEIGHTS){
    // Original ballistic apex cannot reach even the lowest edge of this star's hit region.
    assert.ok(GROUND-HEIGHT/2-480**2/(2*1450)-y>HIT_Y,'tap cannot collect a high star');
    for(const speed of [260,350]){
      let reachable=false;
      for(let x=40;x<500&&!reachable;x++)reachable=fly(y,29,28,speed,x,null,1);
      assert.ok(reachable,`held jump can collect high star ${y} and clear tallest obstacle at ${speed}`);
    }
  }
  const run=newRun();run.status='running';run.y=100;run.spawn=1e6;
  run.stars=[{x:80,y:68,value:50}];advance(run,1/240,1000);
  assert.equal(run.collected,1);assert.equal(run.bonus,25);
  assert.equal(points(run),Math.floor(run.distance/10)+50);
  advance(run,1/240,1000);assert.equal(run.collected,1);assert.equal(run.bonus,25);
});

test('a full held high-star jump can begin after the star is visible on a 320px track',()=>{
  for(const fps of [30,144])for(const speed of [260,350])for(const y of HIGH_HEIGHTS){
    let success=false;
    for(let startX=120;startX<=308&&!success;startX++){
      const run=newRun();run.status='running';run.spawn=1e6;run.distance=(speed-260)*300;run.speed=speed;
      const obstacle={x:600,width:28,height:29,passed:false};run.obstacles=[obstacle];run.stars=[{x:612,y,value:50}];
      let began=false,released=false,landed=false;
      for(let frame=0;frame<fps*5&&run.status==='running';frame++){
        if(!began&&obstacle.x<=startX){
          assert.ok(run.stars[0].x<=320,'input begins only once the star is on-screen');
          pressJump(run);began=true;
        }
        if(began&&!released&&holdPower(run)===1){releaseJump(run);released=true;}
        advance(run,1/fps,320,()=>.5);
        if(released&&run.y===0&&run.velocity===0)landed=true;
        if(obstacle.passed&&landed)break;
      }
      success=released&&landed&&obstacle.passed&&run.status==='running'&&run.collected===1&&run.bonus===25;
    }
    assert.ok(success,`visible anticipation allows full press/hold/collect/clear/land at ${fps}fps ${speed}px/s star ${y}`);
  }
});

test('four real obstacle families stay within their bounded silhouettes and open on gates',()=>{
  assert.deepEqual(OBSTACLE_KINDS,['stone','book','pot','fence'],'the runner has exactly four named ground families');
  const seen = new Set();
  for(let i=0;i<400;i++){
    const run=newRun();run.spawns=i;run.distance=4000;
    const o=spawnObstacle(run,()=>0,320);
    seen.add(o.kind);
    assert.ok(['stone','book','pot','fence'].includes(o.kind),`spawned family ${o.kind} is one of the four`);
    const [wLo,wHi]=OBSTACLE_BOUNDS[o.kind].width,[hLo,hHi]=OBSTACLE_BOUNDS[o.kind].height;
    assert.ok(o.width>=wLo&&o.width<=wHi,`${o.kind} width ${o.width} inside its band`);
    assert.ok(o.height>=hLo&&o.height<=hHi,`${o.kind} height ${o.height} inside its band`);
    assert.ok(o.x===336,'an obstacle first appears just off the right edge of a 320px board');
  }
  assert.equal(seen.size,4,'all four families actually spawn together');
  const early=newRun();early.spawns=0;early.distance=0;
  for(let i=0;i<30;i++){early.spawns=i;const k=spawnObstacle(early,()=>0,320).kind;assert.ok(k==='stone'||k==='book',`before gates only easy kinds, saw ${k}`);}
  const gate=newRun();gate.spawns=2;gate.distance=1500;gate.obstacles=[];
  assert.ok(isHard(spawnObstacle(gate,()=>0,320).kind),'a pot opens by 1500px on its scheduled hard step');
});

test('a densifying generated field leaves a clearable landing window',()=>{
  // Independent playability over real `advance`, not a mirror of the spacing formula: a greedy
  // tap controller that only reacts once an obstacle is on-screen must clear and land through a
  // long, genuinely spawned, densifying field at every advertised speed/frame rate. This is what
  // the short/long breathing and the distance-driven difficulty actually buy a player.
  function autoplay({ fps, speed, seconds = 40 }) {
    const run = newRun(); run.status = 'running';
    run.speed = speed; run.distance = (speed - 260) * 300; // start at the requested speed, with ordinary acceleration
    const random = seeded(17);
    const rewards=[];
    let frames = 0, maxFrames = fps * seconds;
    const startDistance = (speed - 260) * 300; // the pinned starting distance is part of the seeded run
    while (run.status === 'running' && frames < maxFrames && run.distance < startDistance + 26000) {
      const next = run.obstacles.find(item => !item.passed && item.x + item.width >= 0 && item.x <= 320);
      const gap = next ? next.x - PX : 1e9;
      const star = next && run.stars.find(item => Math.abs(item.x - next.x - 12) < 1);
      if (run.holding && holdPower(run) === 1) releaseJump(run);
      if (run.y === 0 && run.velocity === 0 && !run.holding && next) {
        if (star?.value === 50 && next.x <= Math.min(308, 98 + run.speed * .4)) pressJump(run);
        else if (star?.value !== 50 && gap > 30 && gap < 95) jump(run);
      }
      const before=run.spawns;
      advance(run, 1 / fps, 320, random); frames++;
      if(run.spawns!==before)rewards.push(run.stars.at(-1));
    }
    const ordinary=rewards.filter(star=>star.value!==50&&(star.caught||star.x<50));
    return { run, frames, ordinary };
  }
  for (const fps of [30, 60, 144]) for (const speed of [260, 280, 350]) {
    const { run, frames, ordinary } = autoplay({ fps, speed });
    assert.equal(run.status, 'running', `${fps}fps ${speed}px/s never collides through a densifying field`);
    assert.ok(run.cleared >= 4, `${fps}fps ${speed}px/s clears a real sequence of obstacles (${run.cleared})`);
    assert.ok(run.bonus >= 25, `${fps}fps ${speed}px/s collects a held high star within the continuous field`);
    assert.ok(run.collected >= 3, `${fps}fps ${speed}px/s keeps collecting stars (${run.collected})`);
    assert.ok(ordinary.length>=3,'the continuous field actually presented several ordinary rewards');
    assert.ok(ordinary.every(star=>star.caught),`${fps}fps ${speed}px/s: every passed ordinary reward is collected by the normal tap rhythm`);
    assert.ok(frames > 0, `${fps}fps ${speed}px/s really played`);
  }
  // The field test above already proves hard obstacles are followed by a recovery window (the
  // greedy controller clears and lands through real pot/fence steps). One extra independent probe:
  // a hard obstacle never unavoidably chains into a collision within the same visible window.
  const run = newRun(); run.status = 'running'; run.speed = 340; run.distance = (340 - 260) * 300;
  // Use the real first-spawn position. A manually placed obstacle at x=600 while later
  // ones spawn at x=336 creates an artificial overlap unrelated to the generator's gaps.
  run.obstacles = [{ x: 336, width: 22, height: 44, kind: 'pot', passed: false }];
  run.stars = [spawnStar(run.obstacles[0], 2)];
  run.spawn = spawnInterval({ ...run, obstacles: run.obstacles }, () => 0);
  let landings = 0, wasAir = false;
  for (let frame = 0; frame < 60 * 6 && run.status === 'running'; frame++) {
    const next = run.obstacles.find(item => !item.passed && item.x <= 320);
    if (run.y === 0 && next && next.x - PX > 30 && next.x - PX < 95) jump(run);
    wasAir = run.y > 0 || run.velocity > 0;
    advance(run, 1 / 60, 320, () => 0);
    if (wasAir && run.y === 0) landings++;
  }
  assert.equal(run.status, 'running', 'a hard pot leaves a clearable window, not an inevitable collision');
  assert.ok(run.cleared >= 1, 'the pot and whatever followed were cleared without a collision');
  assert.ok(landings >= 1, 'the player landed again after the hard pot');
});

test('three places retain the original theme ink and rectilinear pixel marks',()=>{
  assert.deepEqual(PLACES.map(p=>p.id),['lake','forest','court']);
  for(const night of [false,true]){
    const palette={soft:'#faf9f6',line:'#dedbd4',muted:'#8d8a82',accent:'#cd7855',night};
    for(let index=0;index<3;index++){
      const ink=new Set(),rects=[];
      const ctx={save(){},restore(){},clearRect(){},fillRect(x,y,w,h){ink.add(this.fillStyle);rects.push([x,y,w,h]);}};
      drawScenery(ctx,320,{distance:index*PLACE_LENGTH},palette,true);
      assert.ok([...ink].every(c=>Object.values(palette).includes(c)),'scenery cannot introduce an independent colourful palette');
      assert.ok(rects.some(([x,y,w,h])=>x===0&&y===0&&w===320&&h===230),'the original paper field covers the board');
      assert.ok(rects.some(([,y,w,h])=>w===8&&h===188-y),'the original stepped hill columns remain');
    }
  }
});

test('the scene schedules each of the three places in order and a run really visits all three',()=>{
  const spots=[0,PLACE_LENGTH*.5,PLACE_LENGTH,PLACE_LENGTH*2+PLACE_LENGTH*.5,PLACE_LENGTH*3];
  const seen=[];
  for(const d of spots)seen.push(PLACES[scene({distance:d},false).index].id);
  assert.equal(seen[0],'lake');
  assert.equal(seen[1],'lake');
  assert.equal(seen[2],'forest');
  assert.equal(seen[3],'court');
  assert.equal(seen[4],'lake');
  const run=newRun();run.status='running';run.spawn=1e6;
  const visited=new Set();
  for(let frame=0;frame<60*25;frame++){const i=scene({distance:run.distance},false).index;visited.add(PLACES[i].id);advance(run,1/60,320,()=>.5);}
  assert.equal(visited.size,3,'a long genuine run moves through lake, forest and court');
});

test('every largest obstacle and high star permits visible input, clearance and landing',()=>{
  for(const kind of OBSTACLE_KINDS)for(const fps of [30,60,144])for(const speed of [260,280,350]){
    const [,width]=OBSTACLE_BOUNDS[kind].width,[,height]=OBSTACLE_BOUNDS[kind].height;
    for(const starY of [98,...HIGH_HEIGHTS]){
      let success=false;
      for(let startX=100;startX<=308&&!success;startX+=2){
        const run=newRun();run.status='running';run.spawn=1e6;run.distance=(speed-260)*300;
        const obstacle={kind,x:600,width,height,passed:false};run.obstacles=[obstacle];
        run.stars=[{x:612,y:starY,value:starY<90?50:25}];
        let began=false,released=false,landed=false;
        for(let frame=0;frame<fps*5&&run.status==='running';frame++){
          if(!began&&obstacle.x<=startX){
            assert.ok(run.stars[0].x<=320,'input starts after the star is visible');
            if(starY<90)pressJump(run);else{jump(run);released=true;}began=true;
          }
          if(began&&!released&&holdPower(run)===1){releaseJump(run);released=true;}
          advance(run,1/fps,320,()=>0);
          if(released&&run.y===0&&run.velocity===0)landed=true;
          if(obstacle.passed&&landed)break;
        }
        success=released&&landed&&obstacle.passed&&run.status==='running'&&run.collected===1;
      }
      assert.ok(success,`${kind} ${width}x${height}, star ${starY}, ${fps}fps/${speed}px/s: visible input -> collect/clear/land`);
    }
  }
});

test('actual generated spacing has closer and wider stretches at maximum speed',()=>{
  const run=newRun();run.status='running';run.distance=27000;run.spawn=0;
  const random=seeded(72), intervals=[];let last=0;
  // A wide synthetic board keeps collisions out of this spacing observation, while advance
  // still schedules and generates real obstacles with the maximum-speed difficulty.
  for(let frame=0;frame<60*40;frame++){
    const before=run.spawns;advance(run,1/60,100000,random);
    if(run.spawns!==before){if(last)intervals.push(run.elapsed-last);last=run.elapsed;}
  }
  assert.ok(intervals.length>=15,'observe a sustained generated sequence');
  assert.ok(Math.max(...intervals)-Math.min(...intervals)>.2,'later play retains both closer and wider stretches');
  assert.ok(Math.min(...intervals)>=.9,'later obstacles still leave time for a legal jump and landing');
});

test('ordinary generated stars reward a normal safe tap across a useful timing window',()=>{
  // Use a fixed, ordinary approach window rather than scanning for a pixel-perfect solution.
  // Every successful clearance in this window must also collect its ordinary star.
  for(const kind of OBSTACLE_KINDS)for(const fps of [30,60,144])for(const speed of [260,280,350]){
    const [,width]=OBSTACLE_BOUNDS[kind].width,[,height]=OBSTACLE_BOUNDS[kind].height;
    for(let ordinal=0;ordinal<12;ordinal++){
      if(ordinal%4===3)continue;
      let safe=0;
      for(const approach of [130,140,150,160]){
        const run=newRun();run.status='running';run.spawn=1e6;
        run.distance=(speed-260)*300;run.speed=speed;
        const obstacle={kind,x:336,width,height,passed:false};
        const star=spawnStar(obstacle,ordinal);run.obstacles=[obstacle];run.stars=[star];
        let pressed=false,released=false,held=0,landed=false;
        for(let frame=0;frame<fps*3&&run.status==='running';frame++){
          if(!pressed&&obstacle.x<=approach){pressJump(run);pressed=true;}
          if(pressed&&!released){held+=1/fps;if(held>=.05){releaseJump(run);released=true;}}
          advance(run,1/fps,320,()=>.5);
          if(pressed&&run.y===0&&run.velocity===0)landed=true;
          if(obstacle.passed&&landed)break;
        }
        if(obstacle.passed&&landed&&run.status==='running'){
          safe++;
          assert.equal(run.collected,1,`${kind}, star ${star.y}, ${fps}fps/${speed}px/s, press at ${approach}: safe tap also collects`);
        }
      }
      assert.ok(safe>=2,`${kind}/${fps}/${speed}: ordinary tapping has several safe timings`);
    }
  }
});
