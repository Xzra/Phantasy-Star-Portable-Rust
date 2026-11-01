"""Preflight and import orchestration tests; these do not replace a native UE run."""
import importlib.util
import json
from pathlib import Path
import struct
import sys
import tempfile
import types
import unittest
from unittest.mock import patch

SOURCE = Path(__file__).resolve().parents[1] / "unreal/template/Content/Python/import_phantasy.py"
spec = importlib.util.spec_from_file_location("import_phantasy", SOURCE)
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


def make_kit(root):
    assets = []
    for name, bones, clips in [("Hunter_Sword", 17, 9), ("Hunter_Partisan", 17, 7), ("Hunter_Knuckles", 17, 7), ("ForestCrawler", 19, 0), ("LobbyScenery", 0, 0), ("FieldScenery", 0, 0)]:
        animations = [{"name": "Clip_%d" % i, "duration_seconds": 1.0} for i in range(clips)]
        doc = {"asset": {"version": "2.0"}, "animations": animations}
        if bones:
            doc["skins"] = [{"joints": list(range(bones))}]
        text = json.dumps(doc).encode()
        text += b" " * (-len(text) % 4)
        data = struct.pack("<4sIIII", b"glTF", 2, 28 + len(text), len(text), 0x4E4F534A) + text + struct.pack("<II", 0, 0x004E4942)
        relative = "Assets/%s.glb" % name
        path = root / relative
        path.parent.mkdir(exist_ok=True)
        path.write_bytes(data)
        assets.append({"name": name, "file": relative, "skeletal": bool(bones), "bones": bones, "animations": animations})
    box = {"label": "TestProp", "location_cm": [200, 0, 100], "size_cm": [100, 100, 200], "color": [40, 120, 150]}
    scenes = [{"name": n, "environment": n + "Scenery", "boxes": [box], "markers": [{"label": "Marker", "location_cm": [0, 0, 0]}], "player_start_cm": [0, -550, 100]} for n in ["Lobby", "Field"]]
    data = {"version": 1, "target_unreal": "5.7.4", "gameplay_ported": False, "assets": assets, "scenes": scenes}
    (root / "Data").mkdir()
    (root / "Data/manifest.json").write_text(json.dumps(data))
    (root / "Content/Python").mkdir(parents=True)
    return data


class Object:
    def __init__(self, **props):
        self.props = props
    def set_editor_property(self, key, value):
        self.props[key] = value
    def get_editor_property(self, key):
        return self.props[key]
    def get_path_name(self):
        return "/Transient/" + type(self).__name__


class SkeletalMesh(Object):
    pass


class StaticMesh(Object):
    pass


class AnimSequence(Object):
    pass


class Component(Object):
    def set_static_mesh(self, mesh):
        self.mesh = mesh
    def set_skeletal_mesh_asset(self, mesh):
        self.mesh = mesh
    def set_collision_enabled(self, value):
        self.collision = value
    def set_material(self, index, value):
        self.material = value


class Actor(Object):
    def __init__(self):
        super().__init__()
        self.static_mesh_component = Component()
        self.skeletal_mesh_component = Component()
        self.light_component = Component()
    def set_actor_label(self, label):
        self.label = label
    def set_actor_scale3d(self, value):
        self.scale = value


def fake_unreal(data, *, exists=False, dirty=False, broken=False):
    calls = {"imports": [], "maps": [], "actors": [], "pipelines": []}
    assets_by_name = {a["name"]: a for a in data["assets"]}
    class SoftObjectPath:
        def __init__(self, value):
            self.path = value
    class Parameters(Object):
        def set_editor_property(self, key, value):
            if key == "override_pipelines":
                assert all(isinstance(v, SoftObjectPath) for v in value), "UE expects soft object paths"
            super().set_editor_property(key, value)
    class Pipeline(Object):
        def __init__(self):
            super().__init__(mesh_pipeline=Object(), animation_pipeline=Object(), common_skeletal_meshes_and_animations_properties=Object())
            calls["pipelines"].append(self)
    class Manager:
        @staticmethod
        def get_interchange_manager_scripted():
            return Manager()
        @staticmethod
        def create_source_data(path):
            return path
        def import_asset(self, destination, source, parameters):
            calls["imports"].append(destination)
            item = assets_by_name[Path(source).stem]
            mesh = SkeletalMesh(skeleton=Object()) if item["skeletal"] else StaticMesh()
            return [mesh] + ([] if broken else [AnimSequence() for _ in item["animations"]])
    class Levels:
        def new_level(self, path):
            calls["maps"].append(path)
            return True
        def save_current_level(self):
            return True
        def load_level(self, path):
            calls["loaded"] = path
            return True
    class Actors:
        def spawn_actor_from_class(self, cls, location, rotation):
            actor = Actor()
            calls["actors"].append(actor)
            return actor
    class AssetLibrary:
        @staticmethod
        def does_directory_exist(path):
            return exists
        @staticmethod
        def make_directory(path):
            return True
        @staticmethod
        def save_loaded_asset(obj):
            return True
        @staticmethod
        def set_metadata_tag(*args):
            pass
    class Tools:
        @staticmethod
        def get_asset_tools():
            return Tools()
        def create_asset(self, *args):
            return Object()
    class Materials:
        @staticmethod
        def create_material_expression(*args):
            return Object()
        @staticmethod
        def connect_material_property(*args):
            return True
        @staticmethod
        def recompile_material(*args):
            pass
    ue = types.SimpleNamespace(
        SystemLibrary=types.SimpleNamespace(get_engine_version=lambda: "5.7.4-test"),
        EditorAssetLibrary=AssetLibrary,
        EditorLoadingAndSavingUtils=types.SimpleNamespace(get_dirty_map_packages=lambda: [1] if dirty else [], get_dirty_content_packages=lambda: []),
        EditorActorSubsystem=Actors, LevelEditorSubsystem=Levels,
        get_editor_subsystem=lambda cls: cls(), InterchangeManager=Manager,
        InterchangeGenericAssetsPipeline=Pipeline, ImportAssetParameters=Parameters, SoftObjectPath=SoftObjectPath,
        StaticMesh=StaticMesh, SkeletalMesh=SkeletalMesh, AnimSequence=AnimSequence,
        StaticMeshActor=Actor, SkeletalMeshActor=Actor, TargetPoint=Actor, PlayerStart=Actor, DirectionalLight=Actor, SkyLight=Actor,
        Vector=lambda *args: args, Rotator=lambda **kwargs: kwargs,
        CollisionEnabled=types.SimpleNamespace(QUERY_AND_PHYSICS=1, NO_COLLISION=0),
        load_asset=lambda path: StaticMesh(), AssetToolsHelpers=Tools, Material=Object, MaterialFactoryNew=Object,
        MaterialEditingLibrary=Materials, MaterialExpressionConstant3Vector=Object,
        MaterialProperty=types.SimpleNamespace(MP_BASE_COLOR=0), LinearColor=lambda *args: args,
        log=lambda message: None, log_error=lambda message: None,
    )
    return ue, calls


class UnrealMigrationTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.data = make_kit(self.root)
    def tearDown(self):
        self.temp.cleanup()
    def run_import(self, **options):
        ue, calls = fake_unreal(self.data, **options)
        with patch.dict(sys.modules, unreal=ue), patch.object(module, "__file__", str(self.root / "Content/Python/import_phantasy.py")):
            module.run()
        return calls
    def test_preflight_accepts_consistent_kit_and_restricts_destination(self):
        self.assertEqual(module.read_manifest(self.root), self.data)
        self.assertEqual(module.valid_destination("/Game/PhantasyMigration_2"), "/Game/PhantasyMigration_2")
        for path in ["/Engine/Test", "/Game/../Test", "/Game/PhantasyMigration/Other"]:
            with self.assertRaises(ValueError):
                module.valid_destination(path)
    def test_corrupt_glb_and_escaping_paths_fail_preflight(self):
        asset = self.root / self.data["assets"][0]["file"]
        original = asset.read_bytes()
        asset.write_bytes(b"glTF")
        with self.assertRaises(ValueError):
            module.read_manifest(self.root)
        asset.write_bytes(original)
        self.data["assets"][0]["file"] = "../escape.glb"
        (self.root / "Data/manifest.json").write_text(json.dumps(self.data))
        with self.assertRaises(ValueError):
            module.read_manifest(self.root)
    def test_mismatched_animation_names_fail_preflight(self):
        self.data["assets"][0]["animations"][0]["name"] = "Wrong"
        (self.root / "Data/manifest.json").write_text(json.dumps(self.data))
        with self.assertRaises(ValueError):
            module.read_manifest(self.root)
    def test_existing_namespace_and_unsaved_work_are_preserved(self):
        for options in [{"exists": True}, {"dirty": True}]:
            with self.assertRaises(RuntimeError):
                self.run_import(**options)
            self.assertFalse((self.root / "Saved/PhantasyImportReport.json").exists())
    def test_import_orchestration_records_native_assets_maps_and_shared_skeleton(self):
        calls = self.run_import()
        report = json.loads((self.root / "Saved/PhantasyImportReport.json").read_text())
        self.assertTrue(report["success"])
        self.assertFalse(report["gameplay_ported"])
        self.assertEqual(len(calls["imports"]), 6)
        self.assertEqual(calls["maps"], ["/Game/PhantasyMigration/Maps/Lobby", "/Game/PhantasyMigration/Maps/Field"])
        self.assertEqual(calls["loaded"], calls["maps"][0])
        self.assertIn("skeleton", calls["pipelines"][1].props["common_skeletal_meshes_and_animations_properties"].props)
    def test_import_failure_preserves_partial_results_and_writes_report(self):
        with self.assertRaises(RuntimeError):
            self.run_import(broken=True)
        report = json.loads((self.root / "Saved/PhantasyImportReport.json").read_text())
        self.assertFalse(report["success"])
        self.assertIn("Unexpected mesh/animation count", report["error"])
        self.assertTrue((self.root / "Assets/Hunter_Sword.glb").exists())


if __name__ == "__main__":
    unittest.main()
