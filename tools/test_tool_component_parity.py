#!/usr/bin/env python3
"""Compare per-stack Tool mining speed and harvest decisions with vanilla 26.3."""
import json
try:
    from . import lab, oracle_harness as harness
except ImportError:
    import lab, oracle_harness as harness


def make_input(build):
    tools = [None, '{rules:[]}', '{rules:[],default_mining_speed:0.25f}',
             '{rules:[{blocks:[],speed:9f}],default_mining_speed:0.5f}']
    for speed in (0.125, 0.25, 2.0, 3.5, 9.0):
        for correct in ('true', 'false'):
            for first in ('', '{blocks:"minecraft:stone",correct_for_drops:false},'):
                tools.append('{rules:[' + first + '{blocks:["minecraft:stone","minecraft:oak_log"],'
                             + f'speed:{speed}f,correct_for_drops:{correct}'
                             + '},{blocks:"minecraft:stone",speed:99f,correct_for_drops:true}],default_mining_speed:0.5f}')
    rows = [dict(item=item, block=block, tool=tool)
            for item in ('minecraft:stick', 'minecraft:mace')
            for block in ('minecraft:stone', 'minecraft:oak_log', 'minecraft:dirt')
            for tool in tools]
    path = build / 'inputs.jsonl'
    path.write_text(''.join(json.dumps(row) + '\n' for row in rows), encoding='utf-8')
    return path, len(rows)


def main():
    harness.differential('rust-tool-component-parity', 'VanillaToolComponentOracle',
                         'tool_component_probe', make_input,
                         'Stack Tool overrides: independent first matching speed/harvest rules, '
                         'direct block lists, empty rules, defaults, prototype fallback. '
                         'Native tests cover tags; this harness excludes component removals and durability.',
                         sources=[lab.ROOT / 'crates/horizon-client/src/items.rs',
                                  lab.ROOT / 'crates/horizon-client/src/item_tool.rs'])


if __name__ == '__main__':
    main()
