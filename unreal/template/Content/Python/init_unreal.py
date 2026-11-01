"""Register an explicit import action. Opening the project never rewrites maps."""
import unreal

def _install_menu():
    menus = unreal.ToolMenus.get()
    menu = menus.extend_menu("LevelEditor.MainMenu.Tools")
    entry = unreal.ToolMenuEntry(
        name="Phantasy.ImportMigration",
        type=unreal.MultiBlockType.MENU_ENTRY,
    )
    entry.set_label("Phantasy: Import migration kit")
    entry.set_tool_tip("Import the exported original assets and create fresh editable maps. Existing migration content is preserved.")
    entry.set_string_command(
        unreal.ToolMenuStringCommandType.PYTHON, "", "import import_phantasy; import_phantasy.run()"
    )
    menu.add_menu_entry("Phantasy", entry)
    menus.refresh_all_widgets()

_install_menu()
unreal.log("Phantasy migration kit: Tools > Phantasy: Import migration kit. Read MIGRATION.md first.")
