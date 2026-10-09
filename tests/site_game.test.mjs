import test from 'node:test';
import assert from 'node:assert/strict';
import { newRun, jump, advance, clawdPose, spawnStar, points, BODY } from '../apps/site/clawd-game.mjs';

// Restored original tuning the reachability checks depend on.
const PX = 48, GROUND = 188, HEIGHT = 40, HIT_Y = 24;
const MIN_SPEED = 210, MAX_SPEED = 350;
// Every height the star scheme can produce: the three visible rows, each plus its bounded jitter.
const STAR_ROWS = [98, 118, 138], STAR_JITTER = [0, 5, -4, 3, -2, 6, -5, 4, -3, 2];
const STAR_HEIGHTS = [...new Set(STAR_ROWS.flatMap(base => STAR_JITTER.map(j => base + j)))].sort((a, b) => a - b);
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
  const canonical = { anchor: 0, scaleX: 1, scaleY: 1, eyes: [[20,10,4,10],[56,10,4,10]] };
  for (const status of ['ready', 'paused']) {
    const run = newRun(); run.status = status; run.distance = 100;
    assert.deepEqual(clawdPose(run), canonical, `${status} has no standing scale`);
  }
  const run = newRun(); run.status = 'running';
  assert.deepEqual(Object.keys(clawdPose(run)).sort(), ['anchor','eyes','scaleX','scaleY'], 'no per-leg or per-foot fields remain');
  for (let i = 0; i < 8; i++) advance(run, 1 / 60, 1000);
  const stepping = clawdPose(run);
  assert.equal(stepping.anchor, 0, 'grounded running keeps the feet anchored on the ground');
  jump(run); advance(run, 1 / 60, 1000, () => 0);
  const air = clawdPose(run);
  assert.equal(air.anchor, 0, 'the airborne anchor is the same anchored ground goal');
  assert.ok(air.scaleY > 1, 'the rising body stretches vertically instead of tucking legs');
  assert.ok(air.scaleY > stepping.scaleY, 'the ascent stretches taller than the grounded stride');
  assert.deepEqual(clawdPose(run, true), { anchor: 0, scaleX: 1, scaleY: 1, eyes: air.eyes }, 'reduced motion holds the canonical shape static');
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
  for (const y of sequence) assert.ok(STAR_HEIGHTS.includes(y), `${y} is one of the defined rows plus jitter`);
  // Three consecutive stars sit on three different rows, so the change is visible in a play session.
  assert.equal(spawnStar({ x: 0 }, 0).y, 98);
  assert.equal(spawnStar({ x: 0 }, 1).y, 123);
  assert.equal(spawnStar({ x: 0 }, 2).y, 134);
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
  assert.ok(spawned.every(y => STAR_HEIGHTS.includes(y)), 'every advance-spawned star uses the height scheme');
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
function fly(starY, obstacleH, obstacleW, speed, launchX, trace) {
  const run = newRun(); run.status = 'running';
  // advance() recomputes speed as min(350, 210 + distance/220); seed distance so the requested speed
  // holds from the first step. At 350 (the clamp) any distance past 30800 pins it exactly at 350;
  // at 210 the game keeps accelerating from here, which is the real behaviour, not a downgrade.
  run.speed = speed; run.distance = (speed - 210) * 220;
  const obstacle = { x: 600, width: obstacleW, height: obstacleH, kind: 'stone', passed: false };
  run.obstacles = [obstacle];
  run.stars = [{ x: 612, y: starY }];
  run.spawn = 1e6; // a finite spawn delay far beyond this short trial: no unrelated star can appear
  let launched = false, landed = false, speedHeld = run.speed === speed, minSpeed = run.speed, maxSpeed = run.speed;
  for (let step = 0; step < 200000; step++) {
    if (!launched && obstacle.x <= launchX) { jump(run); launched = true; }
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
  // toward 210; at 210 the run starts at 210 and only accelerates upward, which is the real game.
  const fast = {};
  assert.ok(fly(STAR_HEIGHTS[0], 29, 28, MAX_SPEED, 118, fast), 'a 350px/s run collects, clears and lands cleanly');
  assert.equal(fast.minSpeed, MAX_SPEED, 'the maximum-speed trial never drifted below 350');
  assert.equal(fast.maxSpeed, MAX_SPEED, 'the maximum-speed trial stayed pinned at the 350 clamp');
  const slow = {};
  assert.ok(fly(STAR_HEIGHTS[0], 29, 28, MIN_SPEED, 118, slow), 'a 210px/s run collects, clears and lands cleanly');
  assert.equal(slow.minSpeed, MIN_SPEED, 'the normal-speed trial starts at 210, not a hidden 350');
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
  run.speed = speed; run.distance = (speed - 210) * 220; // hold the requested speed from the first step
  const wide = 100000; // no obstacle can reach Clawd; this isolates ordinary grounded running
  let footPeak = 0, footFloor = 0, scalePeak = 1, scaleFloor = 1;
  for (let i = 0; i < fps * seconds; i++) {
    advance(run, 1 / fps, wide, () => .5);
    if (run.y > 0 || run.velocity > 0) continue; // only ordinary grounded running
    const pose = clawdPose(run);
    footPeak = Math.max(footPeak, pose.anchor);
    footFloor = Math.min(footFloor, pose.anchor);
    scalePeak = Math.max(scalePeak, pose.scaleX, pose.scaleY);
    scaleFloor = Math.min(scaleFloor, pose.scaleX, pose.scaleY);
  }
  return { run, footPeak, footFloor, scalePeak, scaleFloor };
}

test('ordinary running anchors the feet on the ground and never bobs the body', () => {
  // The repeated ground jolt came from a whole-body lift keyed to travelled distance. The repair
  // keeps grounded feet exactly on GROUND: over a real run the pose anchor must hold at 0, never
  // lifting or dropping the body, so no vertical ground beat can exist to jolt it.
  for (const speed of [210, 280, 350]) {
    for (const fps of [30, 60, 144]) {
      const { run, footPeak, footFloor } = flyRun({ fps, speed });
      assert.ok(run.distance > 0, `the ${fps}fps ${speed}px/s run really travelled`);
      assert.equal(footPeak, 0, `${fps}fps ${speed}px/s never lifts the grounded body`);
      assert.equal(footFloor, 0, `${fps}fps ${speed}px/s never drops the grounded body`);
    }
  }
});

test('the grounded stride is a subtle deformation, not a visible bob, at every speed and frame rate', () => {
  // With the feet anchored, the only stride signal is a whole-body scale deform: it must stay small
  // (at most 0.5%) and symmetric, at every accepted speed and frame rate.
  for (const speed of [210, 280, 350]) {
    for (const fps of [30, 60, 144]) {
      const { scalePeak, scaleFloor } = flyRun({ fps, speed });
      assert.ok(scalePeak - 1 <= .005, `${fps}fps ${speed}px/s never deforms past +0.5% (saw ${(scalePeak - 1).toFixed(4)})`);
      assert.ok(1 - scaleFloor <= .005, `${fps}fps ${speed}px/s never deforms past -0.5% (saw ${(1 - scaleFloor).toFixed(4)})`);
    }
  }
  // The stride period is a calm fraction of a second, never a high-frequency shake: one travel
  // period must span a visible stretch of ground, so the deformation cannot alias into vibration.
  const samples = [];
  for (let d = 0; d < 1000; d += 0.5) {
    const r = newRun(); r.status = 'running'; r.distance = d;
    samples.push(clawdPose(r).scaleX);
  }
  let turnarounds = 0;
  for (let i = 1; i < samples.length - 1; i++) {
    if (samples[i] > samples[i - 1] && samples[i] > samples[i + 1]) turnarounds++;
  }
  assert.ok(turnarounds <= 12, `a ~100px travel period holds few extrema over 1000px (saw ${turnarounds})`);
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
