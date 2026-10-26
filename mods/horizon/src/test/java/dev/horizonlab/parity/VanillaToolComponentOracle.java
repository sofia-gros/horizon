package dev.horizonlab.parity;

import com.google.gson.JsonParser;
import com.mojang.serialization.JsonOps;
import java.io.PrintWriter;
import java.nio.file.Files;
import java.nio.file.Path;
import net.minecraft.core.component.DataComponentMap;
import net.minecraft.core.component.DataComponents;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.nbt.TagParser;
import net.minecraft.resources.Identifier;
import net.minecraft.resources.RegistryOps;
import net.minecraft.core.RegistryAccess;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.item.component.Tool;

/** Uses the shipped Tool codec and ItemStack queries, with persistent stack patches. */
public final class VanillaToolComponentOracle {
    public static void main(String[] args) throws Exception {
        net.minecraft.SharedConstants.tryDetectVersion();
        net.minecraft.server.Bootstrap.bootStrap();
        var lookup = RegistryAccess.fromRegistryOfRegistries(BuiltInRegistries.REGISTRY);
        var ops = RegistryOps.create(net.minecraft.nbt.NbtOps.INSTANCE, lookup);
        var defaults = JsonParser.parseString(Files.readString(Path.of(args[0]).getParent()
            .getParent().resolve("horizon-client/26.3/items.json"))).getAsJsonObject();
        // Tag resolution is exercised in native tests; prototypes here are stick and mace.
        for (var name : new String[]{"minecraft:stick", "minecraft:mace"}) {
            var data = defaults.getAsJsonObject(name).getAsJsonObject("minecraft:tool");
            var components = DataComponentMap.builder();
            if (data != null) components.set(DataComponents.TOOL,
                Tool.CODEC.parse(RegistryOps.create(JsonOps.INSTANCE, lookup), data).getOrThrow());
            BuiltInRegistries.ITEM.getValue(Identifier.parse(name)).builtInRegistryHolder().bindComponents(components.build());
        }
        try (var out = new PrintWriter(Files.newBufferedWriter(Path.of(args[1])));
             var lines = Files.lines(Path.of(args[0]))) {
            for (var line : (Iterable<String>) lines::iterator) {
                var row = JsonParser.parseString(line).getAsJsonObject();
                var stack = new ItemStack(BuiltInRegistries.ITEM.getValue(Identifier.parse(row.get("item").getAsString())));
                if (!row.get("tool").isJsonNull()) stack.set(DataComponents.TOOL,
                    Tool.CODEC.parse(ops, TagParser.parseCompoundFully(row.get("tool").getAsString())).getOrThrow());
                var block = BuiltInRegistries.BLOCK.getValue(Identifier.parse(row.get("block").getAsString())).defaultBlockState();
                out.println("[" + Integer.toUnsignedString(Float.floatToRawIntBits(stack.getDestroySpeed(block)))
                    + "," + stack.isCorrectToolForDrops(block) + "]");
            }
        }
    }
}
