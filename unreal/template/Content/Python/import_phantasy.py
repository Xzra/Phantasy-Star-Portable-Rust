"""UE 5.7.4 editor-only import. Creates fresh assets/maps; never ports Rust gameplay.

Run from Tools > Phantasy: Import migration kit. Importing this module in ordinary
Python is safe: all Unreal operations are inside run().
"""
import json
import math
import re
import struct
from pathlib import Path


def read_manifest(root):
    """Preflight all inputs before asking Unreal to create any assets."""
    root = Path(root).resolve()
    path = root / "Data" / "manifest.json"
    if path.stat().st_size > 1024 * 1024:
        raise ValueError("Manifest exceeds 1 MiB")
    data = json.loads(path.read_text(encoding="utf-8-sig"))
    if data.get("version") != 1 or data.get("target_unreal") != "5.7.4":
        raise ValueError("Expected migration manifest version 1 targeting UE 5.7.4")
    if data.get("gameplay_ported") is not False:
        raise ValueError("Unexpected gameplay-port claim in migration data")
    assets = data.get("assets", [])
    if len(assets) != 6:
        raise ValueError("Expected three Hunter variants, Crawler and two scenery assets")
    names = set()
    for asset in assets:
        if not re.fullmatch(r"[A-Za-z][A-Za-z0-9_]*", asset["name"]) or asset["name"] in names:
            raise ValueError("Invalid or duplicate asset name")
        names.add(asset["name"])
        relative = Path(asset["file"])
        file = (root / relative).resolve()
        if relative.is_absolute() or ".." in relative.parts or root not in file.parents:
            raise ValueError("Asset path escapes the migration kit")
        if file.suffix != ".glb" or not file.is_file() or file.stat().st_size > 128 * 1024 * 1024:
            raise ValueError("Missing or oversized GLB: " + str(file))
        with file.open("rb") as stream:
            header = stream.read(20)
            if len(header) != 20:
                raise ValueError("Truncated GLB")
            magic, version, length, json_length, chunk_type = struct.unpack("<4sIIII", header)
            if magic != b"glTF" or version != 2 or length != file.stat().st_size or chunk_type != 0x4E4F534A:
                raise ValueError("Invalid GLB container")
            if json_length > length - 20:
                raise ValueError("Invalid GLB JSON length")
            doc = json.loads(stream.read(json_length).decode("utf-8"))
        actual = [a["name"] for a in doc.get("animations", [])]
        if actual != [a["name"] for a in asset["animations"]]:
            raise ValueError("Animation manifest does not match GLB: " + asset["name"])
        if asset["skeletal"] and len(doc.get("skins", [{}])[0].get("joints", [])) != asset["bones"]:
            raise ValueError("Skeleton joint count mismatch")
    scenes = data.get("scenes", [])
    if [s.get("name") for s in scenes] != ["Lobby", "Field"]:
        raise ValueError("Expected Lobby and Field scenes")
    for scene in scenes:
        if scene.get("environment") not in names or len(scene.get("boxes", [])) > 40:
            raise ValueError("Invalid scene contents")
        for box in scene["boxes"]:
            for key in ["location_cm", "size_cm"]:
                values = box[key]
                if len(values) != 3 or any(not isinstance(v, (float, int)) or not math.isfinite(v) for v in values):
                    raise ValueError("Invalid prop coordinates")
            if any(v <= 0 for v in box["size_cm"]):
                raise ValueError("Prop dimensions must be positive")
            if len(box["color"]) != 3 or any(type(v) is not int or not 0 <= v <= 255 for v in box["color"]):
                raise ValueError("Invalid prop color")
    return data


def valid_destination(destination):
    if not re.fullmatch(r"/Game/PhantasyMigration(?:_[A-Za-z0-9]+)?", destination):
        raise ValueError("Use /Game/PhantasyMigration or /Game/PhantasyMigration_uniqueSuffix")
    return destination


def run(destination="/Game/PhantasyMigration"):
    import unreal

    destination = valid_destination(destination)
    root = Path(__file__).resolve().parents[2]
    data = read_manifest(root)
    version = unreal.SystemLibrary.get_engine_version()
    if not version.startswith("5.7."):
        raise RuntimeError("This kit targets UE 5.7.4. Detected " + version)
    if unreal.EditorAssetLibrary.does_directory_exist(destination):
        raise RuntimeError(
            "Migration content already exists. Your edits were preserved. To import another snapshot, "
            "run import_phantasy.run('/Game/PhantasyMigration_2') in the Python console."
        )
    loading = unreal.EditorLoadingAndSavingUtils
    if loading.get_dirty_map_packages() or loading.get_dirty_content_packages():
        raise RuntimeError("Save or close your current unsaved editor work before importing the migration kit.")
    actors = unreal.get_editor_subsystem(unreal.EditorActorSubsystem)
    levels = unreal.get_editor_subsystem(unreal.LevelEditorSubsystem)
    manager = unreal.InterchangeManager.get_interchange_manager_scripted()
    imported = {}
    shared_skeleton = None
    result = {"engine": version, "destination": destination, "assets": {}, "maps": [], "gameplay_ported": False}
    report = root / "Saved" / "PhantasyImportReport.json"
    report.parent.mkdir(parents=True, exist_ok=True)

    def create_actor(cls, label, location=(0, 0, 0), rotation=None):
        actor = actors.spawn_actor_from_class(cls, unreal.Vector(*location), rotation or unreal.Rotator())
        if actor is None:
            raise RuntimeError("Failed to spawn " + label)
        actor.set_actor_label(label)
        return actor

    def static_actor(label, mesh, location=(0, 0, 0), scale=(1, 1, 1), collision=True, material=None):
        actor = create_actor(unreal.StaticMeshActor, label, location)
        component = actor.static_mesh_component
        component.set_static_mesh(mesh)
        component.set_collision_enabled(unreal.CollisionEnabled.QUERY_AND_PHYSICS if collision else unreal.CollisionEnabled.NO_COLLISION)
        actor.set_actor_scale3d(unreal.Vector(*scale))
        if material is not None:
            component.set_material(0, material)
        return actor

    color_materials = {}

    def color_material(color):
        color = tuple(color)
        if color in color_materials:
            return color_materials[color]
        name = "M_Prop_%02x%02x%02x" % color
        material = unreal.AssetToolsHelpers.get_asset_tools().create_asset(
            name, destination + "/Materials", unreal.Material, unreal.MaterialFactoryNew()
        )
        expression = unreal.MaterialEditingLibrary.create_material_expression(material, unreal.MaterialExpressionConstant3Vector, -240, 0)
        expression.set_editor_property("constant", unreal.LinearColor(*(c / 255.0 for c in color), 1.0))
        unreal.MaterialEditingLibrary.connect_material_property(expression, "", unreal.MaterialProperty.MP_BASE_COLOR)
        unreal.MaterialEditingLibrary.recompile_material(material)
        unreal.EditorAssetLibrary.save_loaded_asset(material)
        color_materials[color] = material
        return material

    try:
        unreal.EditorAssetLibrary.make_directory(destination)
        for item in data["assets"]:
            pipeline = unreal.InterchangeGenericAssetsPipeline()
            pipeline.set_editor_property("asset_type_sub_folders", False)
            pipeline.set_editor_property("use_source_name_for_asset", True)
            mesh_options = pipeline.get_editor_property("mesh_pipeline")
            mesh_options.set_editor_property("import_static_meshes", not item["skeletal"])
            mesh_options.set_editor_property("import_skeletal_meshes", item["skeletal"])
            mesh_options.set_editor_property("combine_static_meshes", True)
            mesh_options.set_editor_property("collision", False)
            common = pipeline.get_editor_property("common_skeletal_meshes_and_animations_properties")
            common.set_editor_property("use_t0_as_ref_pose", False)
            if item["name"].startswith("Hunter_") and shared_skeleton is not None:
                common.set_editor_property("skeleton", shared_skeleton)
            animation = pipeline.get_editor_property("animation_pipeline")
            animation.set_editor_property("import_animations", bool(item["animations"]))
            animation.set_editor_property("import_bone_tracks", True)
            parameters = unreal.ImportAssetParameters()
            parameters.set_editor_property("is_automated", True)
            parameters.set_editor_property("replace_existing", False)
            parameters.set_editor_property("override_pipelines", [unreal.SoftObjectPath(pipeline.get_path_name())])
            source = manager.create_source_data(str(root / item["file"]))
            objects = manager.import_asset(destination + "/Assets/" + item["name"], source, parameters)
            if not objects:
                raise RuntimeError("Interchange returned no assets for " + item["name"])
            meshes = [o for o in objects if isinstance(o, unreal.SkeletalMesh if item["skeletal"] else unreal.StaticMesh)]
            sequences = [o for o in objects if isinstance(o, unreal.AnimSequence)]
            if len(meshes) != 1 or len(sequences) != len(item["animations"]):
                raise RuntimeError("Unexpected mesh/animation count for " + item["name"] + "; see import report and Output Log")
            imported[item["name"]] = meshes[0]
            if item["name"] == "Hunter_Sword":
                shared_skeleton = meshes[0].get_editor_property("skeleton")
            result["assets"][item["name"]] = [o.get_path_name() for o in objects]
            for obj in objects:
                if not unreal.EditorAssetLibrary.save_loaded_asset(obj):
                    raise RuntimeError("Could not save " + obj.get_path_name())
        cube = unreal.load_asset("/Engine/BasicShapes/Cube.Cube")
        if cube is None:
            raise RuntimeError("Engine BasicShapes Cube is unavailable")
        for scene in data["scenes"]:
            path = destination + "/Maps/" + scene["name"]
            unreal.EditorAssetLibrary.make_directory(destination + "/Maps")
            if not levels.new_level(path):
                raise RuntimeError("Could not create " + path)
            # Decorative scenery has no collision; simple native UE boxes provide editable collision.
            static_actor(scene["environment"], imported[scene["environment"]], collision=False)
            static_actor("WalkableFloor", cube, (0, 0, -20), (24 if scene["name"] == "Lobby" else 30, 24 if scene["name"] == "Lobby" else 30, 0.4), material=color_material([45, 65, 68]))
            for box in scene["boxes"]:
                static_actor(box["label"], cube, box["location_cm"], [v / 100 for v in box["size_cm"]], material=color_material(box["color"]))
            for marker in scene["markers"]:
                actor = create_actor(unreal.TargetPoint, marker["label"], marker["location_cm"])
                unreal.EditorAssetLibrary.set_metadata_tag(actor, "PhantasyPortStatus", "Marker only: implement interaction in Blueprint/C++")
            create_actor(unreal.PlayerStart, "PlayerStart", scene["player_start_cm"])
            light = create_actor(unreal.DirectionalLight, "Sun", (0, 0, 1000), unreal.Rotator(pitch=-45, yaw=-30))
            light.light_component.set_editor_property("intensity", 4.0)
            create_actor(unreal.SkyLight, "SkyLight", (0, 0, 500))
            for i, name in enumerate(["Hunter_Sword", "Hunter_Partisan", "Hunter_Knuckles"]):
                actor = create_actor(unreal.SkeletalMeshActor, name + "_EditablePreview", (-350 + i * 350, 0, 0))
                actor.skeletal_mesh_component.set_skeletal_mesh_asset(imported[name])
                actor.skeletal_mesh_component.set_collision_enabled(unreal.CollisionEnabled.NO_COLLISION)
            if not levels.save_current_level():
                raise RuntimeError("Could not save " + path)
            result["maps"].append(path)
        result["success"] = True
        if not levels.load_level(result["maps"][0]):
            raise RuntimeError("Imported maps saved, but Lobby could not be opened")
        unreal.log("Phantasy migration imported. Maps and assets are editable. Gameplay still needs Blueprint/C++ implementation. Read MIGRATION.md.")
    except Exception as error:
        result["success"] = False
        result["error"] = str(error)
        unreal.log_error("Phantasy migration stopped: " + str(error))
        raise
    finally:
        report.write_text(json.dumps(result, indent=2), encoding="utf-8")


if __name__ == "__main__":
    run()
