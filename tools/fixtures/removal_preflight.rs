#![allow(dead_code, unused_variables)]
use std::collections::{HashMap, HashSet};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct BlockPos {
    x: i32,
    y: i32,
    z: i32,
}
fn at(x: i32) -> BlockPos {
    BlockPos { x, y: 1, z: 0 }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct BlockStateId(u32);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Direction {
    East,
    West,
    Up,
    Down,
    North,
    South,
}
impl Direction {
    const ALL: [Self; 6] = [
        Self::East,
        Self::West,
        Self::Up,
        Self::Down,
        Self::North,
        Self::South,
    ];
    fn opposite(self) -> Self {
        match self {
            Self::East => Self::West,
            Self::West => Self::East,
            Self::Up => Self::Down,
            Self::Down => Self::Up,
            Self::North => Self::South,
            Self::South => Self::North,
        }
    }
}
fn axis_of(d: Direction) -> u8 {
    match d {
        Direction::East | Direction::West => 0,
        Direction::Up | Direction::Down => 1,
        _ => 2,
    }
}
fn rel(p: BlockPos, d: Direction) -> BlockPos {
    rel_n(p, d, 1)
}
fn rel_n(mut p: BlockPos, d: Direction, n: i32) -> BlockPos {
    match d {
        Direction::East => p.x += n,
        Direction::West => p.x -= n,
        Direction::Up => p.y += n,
        Direction::Down => p.y -= n,
        Direction::North => p.z -= n,
        Direction::South => p.z += n,
    };
    p
}
enum LevelEvent {
    DropResources { pos: BlockPos, state: BlockStateId },
}
struct Rules {
    block_drops: bool,
}
enum Class {
    StrawBed,
    BaseFire,
    Fire,
    SoulFire,
}
struct Data;
impl Data {
    fn is(&self, _: Class) -> bool {
        false
    }
}
#[derive(Clone, Copy, PartialEq)]
enum PushReaction {
    PushPull,
    Push,
    Popped,
    Immoveable,
}
struct Facts {
    push: PushReaction,
    hardness: f32,
}
#[derive(Clone, Copy)]
struct Named {
    piston_head: BlockStateId,
    piston: BlockStateId,
    sticky_piston: BlockStateId,
    moving_piston: BlockStateId,
    slime_block: BlockStateId,
    honey_block: BlockStateId,
}
struct Resolver {
    piston_pos: BlockPos,
    extending: bool,
    start: BlockPos,
    push_direction: Direction,
    piston_direction: Direction,
    to_push: Vec<BlockPos>,
    to_destroy: Vec<BlockPos>,
}
#[derive(Clone, Copy)]
struct Props {
    extended: (),
    waterlogged: (),
}
#[derive(Clone, Copy)]
struct Table;
impl Table {
    fn bool(&self, _: BlockStateId, _: ()) -> bool {
        false
    }
    fn set_bool(&self, s: BlockStateId, _: (), _: bool) -> BlockStateId {
        s
    }
}
#[derive(Clone, Copy)]
struct B {
    props: Props,
    named: Named,
    table: Table,
}
impl B {
    fn air(&self) -> BlockStateId {
        BlockStateId(0)
    }
    fn is_block(&self, s: BlockStateId, b: BlockStateId) -> bool {
        s == b
    }
    fn block(&self, s: BlockStateId) -> BlockStateId {
        s
    }
    fn is_air(&self, s: BlockStateId) -> bool {
        s.0 == 0
    }
    fn data(&self, _: BlockStateId) -> Data {
        Data
    }
    fn legacy_block(&self, _: ()) -> BlockStateId {
        self.air()
    }
    fn facing(&self, _: BlockStateId) -> Direction {
        Direction::East
    }
    fn has_flag(&self, _: BlockStateId, _: i32) -> bool {
        false
    }
    fn is_waterlogged(&self, _: BlockStateId) -> bool {
        false
    }
    fn facts(&self, s: BlockStateId) -> Facts {
        Facts {
            hardness: 1.,
            push: match s.0 {
                2 | 5 => PushReaction::Immoveable,
                4 => PushReaction::Popped,
                _ => PushReaction::PushPull,
            },
        }
    }
}
trait LevelAccess {
    fn block_state(&self, p: BlockPos) -> BlockStateId;
    fn allow_block_removal(&self, p: BlockPos, s: BlockStateId) -> bool;
}
#[derive(Clone, Debug, PartialEq)]
struct L {
    blocks: HashMap<BlockPos, BlockStateId>,
    denied: HashSet<(BlockPos, BlockStateId)>,
    writes: usize,
}
impl LevelAccess for L {
    fn block_state(&self, p: BlockPos) -> BlockStateId {
        *self.blocks.get(&p).unwrap_or(&BlockStateId(0))
    }
    fn allow_block_removal(&self, p: BlockPos, s: BlockStateId) -> bool {
        !self.denied.contains(&(p, s))
    }
}
impl L {
    fn game_rules(&self) -> Rules {
        Rules { block_drops: true }
    }
    fn set_raw(&mut self, p: BlockPos, s: BlockStateId) -> Option<BlockStateId> {
        self.writes += 1;
        Some(self.blocks.insert(p, s).unwrap_or(BlockStateId(0)))
    }
    fn min_y(&self) -> i32 {
        0
    }
    fn max_y(&self) -> i32 {
        256
    }
    fn has_mod_block_entity(&self, _: BlockPos) -> bool {
        false
    }
    fn is_client_side(&self) -> bool {
        false
    }
    fn game_time(&self) -> i64 {
        10
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
struct MovingPiston {
    moved_state: BlockStateId,
    direction: Direction,
    extending: bool,
    source: bool,
    progress: f32,
    progress_o: f32,
    last_ticked: i64,
    death_ticks: i32,
}
impl MovingPiston {
    fn movement_direction(&self) -> Direction {
        self.direction
    }
}
#[derive(Clone, Default, Debug, PartialEq)]
struct Ticks {
    moving_pistons: Vec<(BlockPos, MovingPiston)>,
    piston_pushes: Vec<(BlockPos, MovingPiston, f32)>,
}
#[derive(Clone, Copy)]
enum RetractAction {
    ClearHead,
    Pull,
    FinishMoving(BlockPos),
}
struct Sim {
    level: Box<L>,
    b: B,
    drops: usize,
    events: usize,
    ticks: Ticks,
}
impl Sim {
    fn emit(&mut self, _: LevelEvent) {
        self.drops += 1;
    }
    // __ALLOWED__
    fn state(&self, p: BlockPos) -> BlockStateId {
        self.level.block_state(p)
    }
    fn level_event(&mut self, _: i32, _: BlockPos, _: i32) {
        self.events += 1;
    }
    fn fluid_at(&self, _: BlockPos) {}
    fn game_event_state(&mut self, _: BlockPos, _: &str, _: BlockStateId) {
        self.events += 1;
    }
    fn set_block_limit(&mut self, p: BlockPos, s: BlockStateId, f: i32, _: i32) -> bool {
        self.chunk_set_block_state(p, s, f).is_some()
    }
    fn set_block(&mut self, p: BlockPos, s: BlockStateId, f: i32) -> bool {
        self.set_block_limit(p, s, f, 512)
    }
    fn set_block_and_update(&mut self, p: BlockPos, s: BlockStateId) -> bool {
        self.set_block(p, s, 3)
    }
    fn update_from_neighbour_shapes(&self, s: BlockStateId, _: BlockPos) -> BlockStateId {
        s
    }
    fn update_or_destroy(&mut self, _: BlockStateId, s: BlockStateId, p: BlockPos, f: i32, n: i32) {
        self.set_block_limit(p, s, f, n);
    }
    fn initial_orientation(&self, _: Option<Direction>, _: Option<Direction>) {}
    fn neighbor_changed_at_oriented(&mut self, _: BlockPos, _: BlockStateId, _: ()) {
        self.events += 1;
    }
    fn is_sticky_piston(&self, s: BlockStateId) -> bool {
        s == self.b.named.sticky_piston
    }
    fn moving_piston(&self, p: BlockPos) -> Option<MovingPiston> {
        self.ticks
            .moving_pistons
            .iter()
            .find(|(at, _)| *at == p)
            .map(|(_, m)| *m)
    }
    fn remove_moving_piston(&mut self, p: BlockPos) -> Option<MovingPiston> {
        let i = self
            .ticks
            .moving_pistons
            .iter()
            .position(|(at, _)| *at == p)?;
        Some(self.ticks.moving_pistons.remove(i).1)
    }
    // __DROP__
    // __RESOLVER__
    // __DESTROY__
    // __CHUNK__
    // __RETRACT__
    // __MOVING__
    // __PISTON__
}
// __DROPPED__
// __HELPER__
fn fixture() -> Sim {
    Sim {
        level: Box::new(L {
            blocks: HashMap::from([
                (at(0), BlockStateId(1)),
                (at(1), BlockStateId(3)),
                (at(2), BlockStateId(3)),
                (at(3), BlockStateId(4)),
            ]),
            denied: HashSet::new(),
            writes: 0,
        }),
        b: B {
            props: Props {
                extended: (),
                waterlogged: (),
            },
            table: Table,
            named: Named {
                piston: BlockStateId(1),
                piston_head: BlockStateId(2),
                moving_piston: BlockStateId(5),
                sticky_piston: BlockStateId(6),
                slime_block: BlockStateId(7),
                honey_block: BlockStateId(8),
            },
        },
        drops: 0,
        events: 0,
        ticks: Ticks::default(),
    }
}
fn moving() -> MovingPiston {
    MovingPiston {
        moved_state: BlockStateId(99),
        direction: Direction::East,
        extending: true,
        source: false,
        progress: 1.,
        progress_o: 0.5,
        last_ticked: 0,
        death_ticks: 0,
    }
}
#[test]
fn denied_destroy_never_drops_or_writes_even_on_retry() {
    let mut s = fixture();
    s.level.denied.insert((at(1), BlockStateId(3)));
    let old = s.level.clone();
    for _ in 0..3 {
        assert!(!s.destroy_block(at(1), true, 512));
    }
    assert_eq!(s.level, old);
    assert_eq!((s.drops, s.events), (0, 0));
}
#[test]
fn allowed_destroy_drops_once() {
    let mut s = fixture();
    assert!(s.destroy_block(at(1), true, 512));
    assert!(!s.destroy_block(at(1), true, 512));
    assert_eq!((s.drops, s.level.writes), (1, 1));
}
#[test]
fn standalone_drop_refuses() {
    let mut s = fixture();
    s.level.denied.insert((at(1), BlockStateId(3)));
    s.drop_resources(BlockStateId(3), at(1));
    assert_eq!(s.drops, 0);
}
#[test]
fn direct_replacement_refuses() {
    let mut s = fixture();
    s.level.denied.insert((at(1), BlockStateId(3)));
    let old = s.level.clone();
    assert_eq!(s.chunk_set_block_state(at(1), BlockStateId(0), 0), None);
    assert_eq!(old, s.level);
}
#[test]
fn real_resolver_checks_extension_sources_and_destroyed_block() {
    for x in 0..4 {
        let mut s = fixture();
        let p = at(x);
        s.level.denied.insert((p, s.state(p)));
        assert!(!s.piston_trigger_event(at(0), BlockStateId(1), 0, 0));
        assert_eq!((s.level.writes, s.drops, s.events), (0, 0, 0));
    }
}
#[test]
fn real_resolver_checks_empty_extension_destination() {
    let mut s = fixture();
    s.level.blocks.remove(&at(3));
    s.level.denied.insert((at(3), BlockStateId(0)));
    assert!(!s.piston_trigger_event(at(0), BlockStateId(1), 0, 0));
}
#[test]
fn normal_retraction_ignores_untouched_two_ahead() {
    let mut s = fixture();
    s.level.blocks.insert(at(1), BlockStateId(2));
    s.level.denied.insert((at(2), BlockStateId(3)));
    assert!(s.piston_trigger_event(at(0), BlockStateId(1), 1, 0));
}
#[test]
fn event_two_no_pull_ignores_untouched_two_ahead() {
    let mut s = fixture();
    s.level.blocks.insert(at(1), BlockStateId(2));
    s.level.denied.insert((at(2), BlockStateId(3)));
    assert!(s.piston_trigger_event(at(0), BlockStateId(6), 2, 0));
}
#[test]
fn sticky_pull_checks_real_payload_and_virtual_head() {
    let mut s = fixture();
    s.level.blocks.insert(at(1), BlockStateId(2));
    let (push, destroy, dir) = s.piston_resolve(at(0), Direction::East, false).unwrap();
    assert_eq!(push, vec![at(2)]);
    assert!(destroy.is_empty());
    assert_eq!(dir, Direction::West);
    s.level.denied.insert((at(2), BlockStateId(3)));
    assert!(!s.piston_trigger_event(at(0), BlockStateId(6), 1, 0));
    assert_eq!(s.state(at(1)), BlockStateId(2));
}
#[test]
fn unpushable_sticky_target_is_not_touched() {
    let mut s = fixture();
    s.level.blocks.insert(at(1), BlockStateId(2));
    s.level.blocks.insert(at(2), BlockStateId(5));
    s.level.denied.insert((at(2), BlockStateId(5)));
    assert!(s.piston_trigger_event(at(0), BlockStateId(6), 1, 0));
}
#[test]
fn moving_payload_finalization_checks_carried_state() {
    let mut s = fixture();
    s.level.blocks.insert(at(2), BlockStateId(5));
    s.ticks.moving_pistons.push((at(2), moving()));
    s.level.denied.insert((at(2), BlockStateId(99)));
    assert!(!s.piston_trigger_event(at(0), BlockStateId(6), 2, 0));
}
#[test]
fn autonomous_tick_and_final_tick_keep_denied_entity_exactly() {
    for tick in [true, false] {
        for carried in [false, true] {
            let mut s = fixture();
            s.level.blocks.insert(at(2), BlockStateId(5));
            s.ticks.moving_pistons.push((at(2), moving()));
            s.level
                .denied
                .insert((at(2), BlockStateId(if carried { 99 } else { 5 })));
            let old = s.ticks.clone();
            let blocks = s.level.clone();
            if tick {
                s.moving_piston_tick(at(2))
            } else {
                s.moving_piston_final_tick(at(2))
            }
            assert_eq!(old, s.ticks);
            assert_eq!(blocks, s.level);
            assert_eq!((s.drops, s.events), (0, 0));
        }
    }
}
#[test]
fn allowed_autonomous_finalization_keeps_carried_block() {
    for tick in [true, false] {
        let mut s = fixture();
        s.level.blocks.insert(at(2), BlockStateId(5));
        s.ticks.moving_pistons.push((at(2), moving()));
        if tick {
            s.moving_piston_tick(at(2))
        } else {
            s.moving_piston_final_tick(at(2))
        }
        assert!(s.ticks.moving_pistons.is_empty());
        assert_eq!(s.state(at(2)), BlockStateId(99));
        assert_eq!(s.level.writes, 1);
    }
}

// Execute the complete command method with a write adapter that can refuse
// either its initial guard or the storage write. Entity cleanup belongs to the
// successful storage operation, not the command's preparation.
mod command {
    use super::{BlockPos, BlockStateId};
    use std::collections::HashMap;
    impl BlockPos {
        pub fn new(x: i32, y: i32, z: i32) -> Self {
            Self { x, y, z }
        }
    }
    mod flags {
        pub const UPDATE_CLIENTS: i32 = 2;
        pub const UPDATE_ALL: i32 = 3;
        pub const UPDATE_SKIP_ALL_SIDEEFFECTS: i32 = 816;
        pub const UPDATE_SKIP_BLOCK_ENTITY_SIDEEFFECTS: i32 = 256;
    }
    type R<T> = Result<T, String>;
    #[derive(Clone)]
    struct BlockInput {
        state: BlockStateId,
        nbt: Option<String>,
    }
    #[derive(Clone)]
    enum Arg {
        Block(BlockInput),
        Other,
    }
    struct Context {
        block: Arg,
        strict: bool,
        destroy: bool,
    }
    impl Context {
        fn get(&self, _: &str) -> Option<&Arg> {
            Some(&self.block)
        }
        fn has(&self, key: &str) -> bool {
            match key {
                "strict" => self.strict,
                "destroy" => self.destroy,
                _ => false,
            }
        }
    }
    fn err(key: &str, _: &[&dyn std::fmt::Display]) -> String {
        key.into()
    }
    struct Text;
    impl Text {
        fn tr(_: &str, _: &[&dyn std::fmt::Display]) -> String {
            String::new()
        }
        fn literal(s: String) -> String {
            s
        }
    }
    struct CommandError;
    impl CommandError {
        fn new(s: String) -> String {
            s
        }
    }
    struct Info {
        name: String,
    }
    struct Res;
    impl Res {
        fn facts(&self) -> &Self {
            self
        }
        fn get(&self, id: BlockStateId) -> Info {
            Info {
                name: id.0.to_string(),
            }
        }
    }
    struct World {
        air: BlockStateId,
        state: BlockStateId,
        block_entities: HashMap<(i32, i32, i32), String>,
    }
    impl World {
        fn block(&self, _: i32, _: i32, _: i32) -> BlockStateId {
            self.state
        }
    }
    struct CommandLevel<'a> {
        world: &'a mut World,
        denied: bool,
        failed: bool,
    }
    impl CommandLevel<'_> {
        fn allow_block_removal(&self, _: BlockPos, _: BlockStateId) -> bool {
            !self.denied
        }
    }
    struct Behaviour;
    impl Behaviour {
        fn can_remove_block(&self, l: &mut CommandLevel<'_>, _: &mut (), p: BlockPos) -> bool {
            l.allow_block_removal(p, l.world.state)
        }
        fn set_block(
            &self,
            l: &mut CommandLevel<'_>,
            _: &mut (),
            p: BlockPos,
            s: BlockStateId,
            _: i32,
        ) -> bool {
            if l.failed || !l.allow_block_removal(p, l.world.state) {
                return false;
            }
            l.world.state = s;
            l.world.block_entities.remove(&(p.x, p.y, p.z));
            true
        }
    }
    struct Game {
        world_debug: bool,
        loaded: usize,
        world: World,
        res: Res,
        denied: bool,
        failed: bool,
        stopped: usize,
    }
    impl Game {
        fn with_level<T>(
            &mut self,
            f: impl FnOnce(&Behaviour, &mut CommandLevel<'_>, &mut ()) -> T,
        ) -> T {
            f(
                &Behaviour,
                &mut CommandLevel {
                    world: &mut self.world,
                    denied: self.denied,
                    failed: self.failed,
                },
                &mut (),
            )
        }
        fn jukebox_removed(&mut self, _: [i32; 3]) {
            self.stopped += 1;
        }
        fn process_level_events(&mut self) -> R<()> {
            Ok(())
        }
        fn new_block_entity(&self, _: &str) -> Option<String> {
            None
        }
        fn block_pos_arg(&self, _: &Context, _: &str) -> R<[i32; 3]> {
            Ok([0, 1, 0])
        }
        fn block_state(&self, b: &BlockInput) -> R<BlockStateId> {
            Ok(b.state)
        }
        fn block_input_state(
            &self,
            _: [i32; 3],
            _: &BlockInput,
            s: BlockStateId,
            _: bool,
        ) -> R<BlockStateId> {
            Ok(s)
        }
        fn destroy_block_dropping(&mut self, _: [i32; 3]) -> R<()> {
            panic!("denied command reached destroy")
        }
        fn feedback(&self, _: String) {}
        fn load_block_entity_nbt(&mut self, _: [i32; 3], _: &str) -> bool {
            self.loaded += 1;
            self.world.block_entities.clear();
            true
        }
        // __SETBLOCK__
        // __COMMAND__
    }
    fn fixture() -> Game {
        Game {
            world_debug: false,
            loaded: 0,
            world: World {
                air: BlockStateId(0),
                state: BlockStateId(1),
                block_entities: HashMap::from([((0, 1, 0), "valuable inventory".into())]),
            },
            res: Res,
            denied: false,
            failed: false,
            stopped: 0,
        }
    }
    #[test]
    fn denied_setblock_fill_keeps_inventory_for_strict_and_normal() {
        for strict in [false, true] {
            let mut g = fixture();
            g.denied = true;
            assert_eq!(g.place_state([0, 1, 0], BlockStateId(2), strict), Ok(false));
            assert_eq!(g.world.state, BlockStateId(1));
            assert_eq!(
                g.world.block_entities.get(&(0, 1, 0)).unwrap(),
                "valuable inventory"
            );
            assert_eq!(g.stopped, 0);
        }
    }
    #[test]
    fn late_write_failure_also_preserves_command_inventory() {
        for strict in [false, true] {
            let mut g = fixture();
            g.failed = true;
            assert_eq!(g.place_state([0, 1, 0], BlockStateId(2), strict), Ok(false));
            assert_eq!(g.world.state, BlockStateId(1));
            assert_eq!(g.world.block_entities.len(), 1);
            assert_eq!(g.stopped, 0);
        }
    }
    #[test]
    fn allowed_command_cleans_entity_only_after_success() {
        let mut g = fixture();
        assert_eq!(g.place_state([0, 1, 0], BlockStateId(2), false), Ok(true));
        assert_eq!(g.world.state, BlockStateId(2));
        assert!(g.world.block_entities.is_empty());
        assert_eq!(g.stopped, 1);
    }

    #[test]
    fn denied_setblock_never_loads_nbt_or_runs_destroy() {
        for same in [false, true] {
            for strict in [false, true] {
                for destroy in [false, true] {
                    let mut g = fixture();
                    g.denied = true;
                    let c = Context {
                        block: Arg::Block(BlockInput {
                            state: BlockStateId(if same { 1 } else { 2 }),
                            nbt: Some("{Items:[]}".into()),
                        }),
                        strict,
                        destroy,
                    };
                    assert!(g.cmd_setblock(&c).is_err());
                    assert_eq!(g.loaded, 0);
                    assert_eq!(g.world.block_entities.len(), 1);
                }
            }
        }
    }
    #[test]
    fn failed_state_change_does_not_load_other_blocks_nbt() {
        let mut g = fixture();
        g.failed = true;
        let c = Context {
            block: Arg::Block(BlockInput {
                state: BlockStateId(2),
                nbt: Some("{Items:[]}".into()),
            }),
            strict: false,
            destroy: false,
        };
        assert!(g.cmd_setblock(&c).is_err());
        assert_eq!(g.loaded, 0);
        assert_eq!(g.world.block_entities.len(), 1);
    }
}

mod callbacks {
    use super::{BlockPos, BlockStateId};
    // __SNAPSHOT__
    struct CallbackBehaviour;
    impl CallbackBehaviour {
        fn can_remove_block(&self, l: &mut CallbackLevel, _: &mut (), _: BlockPos) -> bool {
            !l.denied
        }
    }
    struct CallbackLevel {
        denied: bool,
    }
    impl CallbackLevel {
        fn allow_block_removal(&self, _: BlockPos, _: BlockStateId) -> bool {
            !self.denied
        }
    }
    #[derive(Default)]
    struct Pending {
        clears: usize,
    }
    impl Pending {
        fn clear(&mut self, _: [i32; 3]) {
            self.clears += 1;
        }
    }
    struct Game {
        current: BlockRemovalSnapshot,
        denied: bool,
        pending: Pending,
        effects: usize,
    }
    impl Game {
        fn block_removal_snapshot(&self, _: [i32; 3]) -> BlockRemovalSnapshot {
            self.current.clone()
        }
        fn with_level<T>(
            &mut self,
            f: impl FnOnce(&CallbackBehaviour, &mut CallbackLevel, &mut ()) -> T,
        ) -> T {
            f(
                &CallbackBehaviour,
                &mut CallbackLevel {
                    denied: self.denied,
                },
                &mut (),
            )
        }
        fn player_after_native(&mut self, snapshot: BlockRemovalSnapshot) -> Result<(), ()> {
            let pos = [0, 1, 0];
            let state = BlockStateId(1);
            // __PLAYER_CHECK__
            self.effects += 1;
            Ok(())
        }
        fn explosion_after_native(&mut self, snapshot: BlockRemovalSnapshot) -> Result<(), ()> {
            let pos = [0, 1, 0];
            let state = BlockStateId(1);
            // __BLAST_CHECK__
            self.effects += 1;
            Ok(())
        }
    }
    fn snapshot() -> BlockRemovalSnapshot {
        BlockRemovalSnapshot {
            moving: vec![],
            wire: vec![],
            state: BlockStateId(1),
            live: Some(5),
            saved: Some(6),
            sources: vec![7],
            name: Some("safe".into()),
        }
    }
    #[test]
    fn callback_changes_to_any_inventory_evidence_abort_downstream() {
        for explosion in [false, true] {
            for field in 0..6 {
                let mut g = Game {
                    current: snapshot(),
                    denied: false,
                    pending: Pending::default(),
                    effects: 0,
                };
                match field {
                    0 => g.current.state = BlockStateId(2),
                    1 => g.current.live = Some(9),
                    2 => g.current.saved = None,
                    3 => g.current.sources.push(9),
                    4 => g.current.name = None,
                    _ => g.denied = true,
                };
                if explosion {
                    g.explosion_after_native(snapshot()).unwrap()
                } else {
                    g.player_after_native(snapshot()).unwrap()
                }
                assert_eq!(g.effects, 0);
                assert_eq!(g.pending.clears, usize::from(explosion));
            }
        }
    }
    #[test]
    fn stable_allowed_callbacks_continue() {
        for explosion in [false, true] {
            let mut g = Game {
                current: snapshot(),
                denied: false,
                pending: Pending::default(),
                effects: 0,
            };
            if explosion {
                g.explosion_after_native(snapshot()).unwrap()
            } else {
                g.player_after_native(snapshot()).unwrap()
            }
            assert_eq!(g.effects, 1);
            assert_eq!(g.pending.clears, 0);
        }
    }
}

#[test]
fn direct_moving_shell_paths_preserve_denied_carried_state() {
    for route in 0..3 {
        let mut s = fixture();
        s.level.blocks.insert(at(2), BlockStateId(5));
        s.ticks.moving_pistons.push((at(2), moving()));
        s.level.denied.insert((at(2), BlockStateId(99)));
        let old = s.ticks.clone();
        let blocks = s.level.clone();
        match route {
            0 => {
                s.drop_resources(BlockStateId(5), at(2));
            }
            1 => {
                assert!(!s.destroy_block(at(2), true, 512));
            }
            _ => {
                assert!(s.chunk_set_block_state(at(2), BlockStateId(0), 0).is_none());
            }
        }
        assert_eq!(s.ticks, old);
        assert_eq!(s.level, blocks);
        assert_eq!((s.drops, s.events), (0, 0));
    }
}

mod raw_wire {
    use std::sync::Arc;
    #[derive(Clone, Debug, Default, PartialEq)]
    struct ItemStack {
        extra: Option<Box<StackData>>,
    }
    impl ItemStack {
        fn data(&self) -> Option<&StackData> {
            self.extra.as_deref()
        }
    }
    #[derive(Clone, Debug, Default, PartialEq)]
    struct StackData {
        wire: Wire,
        container: Vec<(u8, ItemStack)>,
        charged: Vec<ItemStack>,
        bundle_contents: Vec<ItemStack>,
        pot_decorations: Option<Box<[Option<ItemStack>; 4]>>,
    }
    // __WIRE__
    // __WIRE_HELPER__
    fn item(byte: u8) -> ItemStack {
        ItemStack {
            extra: Some(Box::new(StackData {
                wire: Wire::new(ItemStack::default(), Arc::from([byte])),
                ..Default::default()
            })),
        }
    }
    #[test]
    fn raw_wire_identity_catches_bytes_and_baseline_changes() {
        let a = item(1);
        let clone = a.clone();
        assert!(
            a.data()
                .unwrap()
                .wire
                .same_snapshot(&clone.data().unwrap().wire)
        );
        for b in [item(1), item(2)] {
            assert_eq!(a, b);
            assert!(
                !a.data()
                    .unwrap()
                    .wire
                    .same_snapshot(&b.data().unwrap().wire)
            );
        }
    }
    #[test]
    fn nested_wire_changes_are_present_in_removal_snapshot() {
        for kind in 0..4 {
            let mut a = ItemStack::default();
            let mut data = StackData::default();
            match kind {
                0 => data.container.push((0, item(1))),
                1 => data.charged.push(item(1)),
                2 => data.bundle_contents.push(item(1)),
                _ => data.pot_decorations = Some(Box::new([Some(item(1)), None, None, None])),
            };
            a.extra = Some(Box::new(data));
            let mut b = a.clone();
            let data = b.extra.as_mut().unwrap();
            match kind {
                0 => data.container[0].1 = item(2),
                1 => data.charged[0] = item(2),
                2 => data.bundle_contents[0] = item(2),
                _ => data.pot_decorations.as_mut().unwrap()[0] = Some(item(2)),
            };
            assert_eq!(a, b);
            let (mut left, mut right) = (vec![], vec![]);
            removal_stack_wire(&a, &mut left);
            removal_stack_wire(&b, &mut right);
            assert!(left != right);
        }
    }
}
