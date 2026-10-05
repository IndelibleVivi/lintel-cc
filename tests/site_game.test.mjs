import test from 'node:test';
import assert from 'node:assert/strict';
import { newRun, jump, advance } from '../apps/site/clawd-game.mjs';

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

test('four legs alternate during running and tuck during a jump', async () => {
  const { clawdPose } = await import('../apps/site/clawd-game.mjs');
  const run = newRun(); run.status = 'running';
  const first = clawdPose(run);
  for (let i=0;i<8;i++) advance(run,1/60,1000);
  const next = clawdPose(run);
  assert.equal(next.legs.length,4);
  assert.notDeepEqual(first.legs,next.legs,'legs must move, not slide with a static silhouette');
  assert.notEqual(next.legs[0].length,next.legs[1].length,'opposing leg pairs alternate');
  jump(run); advance(run,1/60,1000);
  assert.ok(clawdPose(run).legs.every(leg=>leg.length===6),'airborne feet tuck up');
});

test('timed jumps collect stars, clear obstacles and score each star only once', async () => {
  const { points } = await import('../apps/site/clawd-game.mjs');
  const run = newRun();run.status='running';
  for(let i=0;i<1200 && run.collected<3;i++){
    const obstacle=run.obstacles.find(item=>item.x+item.width>=48);
    if(obstacle && obstacle.x<135 && run.y===0)jump(run);
    advance(run,1/60,720,()=>.5);
    assert.equal(run.status,'running','a correctly timed jump must clear the obstacle');
  }
  assert.equal(run.collected,3);
  assert.equal(points(run),Math.floor(run.distance/10)+75);
  assert.ok(run.cleared>=2);
  const caught=run.collected;advance(run,1/60,720,()=>.5);assert.equal(run.collected,caught);
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
