#!/usr/bin/env python3
"""Execute production removal/dispatch code with a tiny in-memory host adapter.

No downloaded gameplay resources are required. This is not a full piston/world
simulation: resource facts/storage are test adapters; resolver and moving-entity lifecycle are production code.
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
resolver = pistons[pistons.index('    fn is_pushable('):pistons.index('    fn moving_piston_state(')]
resolver = resolver.replace('super::data::state_flags::BLOCK_ENTITY', '0').replace('super::query::axis_of', 'axis_of')
retract = pistons[pistons.index('    fn piston_retract_action('):pistons.index('    /// `PistonBaseBlock.triggerEvent`')]
# Keep the complete autonomous tick/finalTick implementations.
moving = pistons[pistons.index('    fn moving_piston_removal_allowed('):pistons.index('    fn is_sticky_piston(')]
moving = moving[:moving.index('    // ----', moving.index('fn moving_piston_tick('))] if '    // ----' in moving[moving.index('fn moving_piston_tick('):] else moving
moving = moving.replace('super::flags::UPDATE_LIMIT','512')
piston = pistons[pistons.index('    fn piston_trigger_event('):pistons.index('        // A pull the level refused')] + 'true\n}\n'
helper = (LEVEL / 'removal.rs').read_text().split('pub(super) fn preflight', 1)[1]
helper = 'fn preflight' + helper
helper = helper.replace('super::LevelAccess', 'LevelAccess').replace('horizon_core::position::BlockPos', 'BlockPos')
piston = piston.replace('super::removal::preflight', 'preflight')
commands = (ROOT / 'crates/horizon-client/src/game/commands.rs').read_text()
command = commands[commands.index('    pub(super) fn place_state('):commands.index('    fn cmd_setblock(')]
command = command.replace('pub(super)', 'pub').replace('horizon_world::level::LevelAccess::allow_block_removal', 'CommandLevel::allow_block_removal')
command = command.replace('horizon_core::position::BlockPos', 'BlockPos').replace('horizon_world::level::flags::', 'flags::')
# Keep the integration checks adjacent to the executable production excerpts.
client = (ROOT / 'crates/horizon-client/src/game.rs').read_text().split('fn break_block_as_player(', 1)[1]
assert client.index('level.allow_block_removal(') < client.index('self.pending.record(')
blast = (ROOT / 'crates/horizon-client/src/game/explosion.rs').read_text().split('fn explosion_hit_block(', 1)[1]
assert blast.index('LevelAccess::allow_block_removal(') < blast.index('self.pending.record(')
assert client.index('self.native_destroy_block(') < client.index('self.block_removal_snapshot(pos) != snapshot') < client.index('self.pad_rumble(')
assert blast.index('self.native_block_drops(') < blast.index('self.block_removal_snapshot(pos) != snapshot') < blast.index('for mut stack in drops')
assert 'self.world.block_entities.remove(' not in command
assert command.index('if !changed') < command.index('self.jukebox_removed(')
player_check = client[client.index('        if self.block_removal_snapshot(pos) != snapshot'):client.index('        self.pad_rumble(')]
blast_check = blast[blast.index('            if self.block_removal_snapshot(pos) != snapshot'):blast.index('            for mut stack in drops')]
for old, new in [('horizon_world::level::LevelAccess::allow_block_removal','CallbackLevel::allow_block_removal'),('horizon_core::position::BlockPos','BlockPos')]:
    blast_check = blast_check.replace(old,new)
game = (ROOT / 'crates/horizon-client/src/game.rs').read_text()
snapshot = game[game.index('#[derive(PartialEq)]\nstruct BlockRemovalSnapshot'):game.index("impl LevelAccess for GameLevel<'_>")]
snapshot = snapshot.replace('#[derive(PartialEq)]', '#[derive(Clone,PartialEq)]').replace('horizon_world::nbt::Tag','u32')
player = (LEVEL / 'hooks.rs').read_text().split('fn player_destroy(', 1)[1]
assert player.index('allow_block_removal(') < player.index('let data = b.data(state);')
source = (ROOT / 'tools/fixtures/removal_preflight.rs').read_text()

for key, value in [('SNAPSHOT',snapshot),('PLAYER_CHECK',player_check),('BLAST_CHECK',blast_check),('COMMAND',command),('RETRACT',retract),('MOVING',moving),('DROP',drop),('RESOLVER',resolver),('DESTROY',destroy),('CHUNK',chunk),('PISTON',piston),('HELPER',helper)]:
    source = source.replace('// __'+key+'__',value)
with tempfile.TemporaryDirectory(prefix='removal-preflight-') as directory:
    path = Path(directory)
    (path/'test.rs').write_text(source)
    subprocess.run(['rustc','--edition=2024','--test',str(path/'test.rs'),'-o',str(path/'test')],check=True)
    subprocess.run([str(path/'test')],check=True)
