#!/usr/bin/env python3
"""Execute production removal/dispatch code with a tiny in-memory host adapter.

No downloaded gameplay resources are required. This is not a full piston/world
simulation: the resolver and moving-entity lookup are supplied test adapters.
"""
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
LEVEL = ROOT / 'crates/horizon-world/src/level'
update = (LEVEL / 'update.rs').read_text()
pistons = (LEVEL / 'pistons.rs').read_text()
destroy = update[update.index('    pub fn destroy_block('):update.index('    pub fn drop_resources(')]
drop = update[update.index('    pub fn drop_resources('):update.index('        // `MovingPistonBlock.getDrops`')] + 'self.drops += 1;\n}\n'
chunk = update[update.index('    fn chunk_set_block_state('):update.index('        self.level.observe_block_changed(')] + 'Some(old)\n}\n'
resolver = pistons[pistons.index('    fn resolver_state('):pistons.index('    fn add_block_line(')]
piston = pistons[pistons.index('    fn piston_trigger_event('):pistons.index('        let extended_state =')] + 'true\n}\n'
helper = (LEVEL / 'removal.rs').read_text().split('pub(super) fn preflight', 1)[1]
helper = 'fn preflight' + helper
helper = helper.replace('super::LevelAccess', 'LevelAccess').replace('horizon_core::position::BlockPos', 'BlockPos')
piston = piston.replace('super::removal::preflight', 'preflight')
# Keep the integration checks adjacent to the executable production excerpts.
client = (ROOT / 'crates/horizon-client/src/game.rs').read_text().split('fn break_block_as_player(', 1)[1]
assert client.index('level.allow_block_removal(') < client.index('self.pending.record(')
blast = (ROOT / 'crates/horizon-client/src/game/explosion.rs').read_text().split('fn explosion_hit_block(', 1)[1]
assert blast.index('LevelAccess::allow_block_removal(') < blast.index('self.pending.record(')
player = (LEVEL / 'hooks.rs').read_text().split('fn player_destroy(', 1)[1]
assert player.index('allow_block_removal(') < player.index('let data = b.data(state);')
source = r'''
#![allow(dead_code,unused_variables)]
use std::collections::{HashMap,HashSet};
#[derive(Clone,Copy,Debug,PartialEq,Eq,Hash)] struct BlockPos(i32);
#[derive(Clone,Copy,Debug,PartialEq,Eq,Hash)] struct BlockStateId(u32);
#[derive(Clone,Copy)] struct Direction(i32);
fn rel(p:BlockPos,d:Direction)->BlockPos {BlockPos(p.0+d.0)}
fn rel_n(p:BlockPos,d:Direction,n:i32)->BlockPos {BlockPos(p.0+d.0*n)}
enum Class {StrawBed,BaseFire,Fire,SoulFire}
struct Data; impl Data{fn is(&self,_:Class)->bool{false}}
#[derive(Clone,Copy)] struct Named{piston_head:BlockStateId}
struct Resolver{extending:bool,piston_pos:BlockPos,piston_direction:Direction}
#[derive(Clone,Copy)] struct B{props:(),named:Named}
impl B{fn air(&self)->BlockStateId{BlockStateId(0)} fn is_block(&self,s:BlockStateId,b:BlockStateId)->bool{s==b} fn is_air(&self,s:BlockStateId)->bool{s.0==0} fn data(&self,_:BlockStateId)->Data{Data}
fn legacy_block(&self,_:())->BlockStateId{BlockStateId(0)} fn facing(&self,_:BlockStateId)->Direction{Direction(1)}}
trait LevelAccess{fn block_state(&self,p:BlockPos)->BlockStateId;fn allow_block_removal(&self,p:BlockPos,s:BlockStateId)->bool;}
#[derive(Clone,Debug,PartialEq,Eq)] struct L{blocks:HashMap<BlockPos,BlockStateId>,denied:HashSet<(BlockPos,BlockStateId)>,writes:usize}
impl LevelAccess for L{fn block_state(&self,p:BlockPos)->BlockStateId{*self.blocks.get(&p).unwrap_or(&BlockStateId(0))}
fn allow_block_removal(&self,p:BlockPos,s:BlockStateId)->bool{!self.denied.contains(&(p,s))}}
impl L{fn set_raw(&mut self,p:BlockPos,s:BlockStateId)->Option<BlockStateId>{self.writes+=1;Some(self.blocks.insert(p,s).unwrap_or(BlockStateId(0)))}}
#[derive(Clone,Copy)] struct Moving{moved_state:BlockStateId}
struct Sim{level:Box<L>,b:B,drops:usize,events:usize,plan:Option<(Vec<BlockPos>,Vec<BlockPos>,Direction)>,moving:HashMap<BlockPos,Moving>}
impl Sim{
fn state(&self,p:BlockPos)->BlockStateId{self.level.block_state(p)}
fn level_event(&mut self,_:i32,_:BlockPos,_:i32){self.events+=1;}
// __DROP__
fn fluid_at(&self,_:BlockPos){}
fn game_event_state(&mut self,_:BlockPos,_:&str,_:BlockStateId){self.events+=1;}
fn set_block_limit(&mut self,p:BlockPos,s:BlockStateId,f:i32,_:i32)->bool{self.chunk_set_block_state(p,s,f).is_some()}
fn piston_resolve(&self,_:BlockPos,_:Direction,_:bool)->Option<(Vec<BlockPos>,Vec<BlockPos>,Direction)>{self.plan.clone()}
fn moving_piston(&self,p:BlockPos)->Option<Moving>{self.moving.get(&p).copied()}
// __RESOLVER__
// __DESTROY__
// __CHUNK__
// __PISTON__
}
// __HELPER__
fn fixture()->Sim{Sim{level:Box::new(L{blocks:HashMap::from([(BlockPos(0),BlockStateId(1)),(BlockPos(1),BlockStateId(2)),(BlockPos(2),BlockStateId(3)),(BlockPos(3),BlockStateId(4))]),denied:HashSet::new(),writes:0}),b:B{props:(),named:Named{piston_head:BlockStateId(2)}},drops:0,events:0,plan:Some((vec![BlockPos(1),BlockPos(2)],vec![BlockPos(3)],Direction(1))),moving:HashMap::new()}}
#[test]fn denied_destroy_never_drops_or_writes_even_on_retry(){let mut s=fixture();s.level.denied.insert((BlockPos(1),BlockStateId(2)));let old=s.level.clone();for _ in 0..3{assert!(!s.destroy_block(BlockPos(1),true,512));}assert_eq!(s.level,old);assert_eq!((s.drops,s.events),(0,0));}
#[test]fn allowed_destroy_drops_once_and_repeated_air_break_is_empty(){let mut s=fixture();assert!(s.destroy_block(BlockPos(1),true,512));assert!(!s.destroy_block(BlockPos(1),true,512));assert_eq!((s.drops,s.level.writes),(1,1));}
#[test]fn standalone_drop_path_cannot_duplicate_guarded_block(){let mut s=fixture();s.level.denied.insert((BlockPos(1),BlockStateId(2)));s.drop_resources(BlockStateId(2),BlockPos(1));s.drop_resources(BlockStateId(2),BlockPos(1));assert_eq!((s.drops,s.level.writes),(0,0));}
#[test]fn direct_replacement_refuses_before_storage(){let mut s=fixture();s.level.denied.insert((BlockPos(1),BlockStateId(2)));let old=s.level.clone();assert_eq!(s.chunk_set_block_state(BlockPos(1),BlockStateId(0),0),None);assert_eq!(old,s.level);}
#[test]fn piston_checks_sources_destinations_destroy_and_base_before_effects(){for a in [0,1,2]{for at in 0..4{let mut s=fixture();let pos=BlockPos(at);s.level.denied.insert((pos,s.state(pos)));let old=s.level.clone();assert!(!s.piston_trigger_event(BlockPos(0),BlockStateId(1),a,0));assert_eq!(old,s.level);assert_eq!((s.drops,s.events),(0,0));}}}
#[test]fn retract_checks_projected_moving_state(){let mut s=fixture();s.moving.insert(BlockPos(2),Moving{moved_state:BlockStateId(99)});s.level.denied.insert((BlockPos(2),BlockStateId(99)));assert!(!s.piston_trigger_event(BlockPos(0),BlockStateId(1),1,0));assert_eq!((s.level.writes,s.drops,s.events),(0,0,0));}
#[test]fn retraction_resolves_head_without_mutating_it(){let s=fixture();let mut r=Resolver{extending:false,piston_pos:BlockPos(0),piston_direction:Direction(1)};assert_eq!(s.resolver_state(&r,BlockPos(1)),BlockStateId(0));assert_eq!(s.state(BlockPos(1)),BlockStateId(2));assert_eq!(s.resolver_state(&r,BlockPos(2)),BlockStateId(3));r.extending=true;assert_eq!(s.resolver_state(&r,BlockPos(1)),BlockStateId(2));}
#[test]fn unguarded_piston_preflight_accepts(){let mut s=fixture();assert!(s.piston_trigger_event(BlockPos(0),BlockStateId(1),0,0));}
'''
for key, value in [('DROP',drop),('RESOLVER',resolver),('DESTROY',destroy),('CHUNK',chunk),('PISTON',piston),('HELPER',helper)]:
    source = source.replace('// __'+key+'__',value)
with tempfile.TemporaryDirectory(prefix='removal-preflight-') as directory:
    path = Path(directory)
    (path/'test.rs').write_text(source)
    subprocess.run(['rustc','--edition=2024','--test',str(path/'test.rs'),'-o',str(path/'test')],check=True)
    subprocess.run([str(path/'test')],check=True)
