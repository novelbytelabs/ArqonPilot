# AGOrg Interface Tutorial

This tutorial provides a step-by-step guide to using the **AGOrg** (Artificial General Organization) tab in Arqon Pilot. 

An **AGOrg** is the top-level project container in the Arqon ecosystem. It orchestrates multiple **AGOs** (Artificial General Organisms), which are individual repositories (like `ArqonCore`, `ArqonBus`, etc.), as well as **nested AGOrgs** (sub-organizations).

---

## Part 1: Creating an AGOrg Project

The **Create AGOrg Project** panel is your starting point for onboarding a new project ecosystem or sub-organization into Arqon Pilot. This flow allows you to define the organizational root, discover nested structures, and set the operational scope in one sequence.

### Step 1: Access the AGOrg Tab
1.  Open the Arqon Pilot Control Panel (usually at `http://localhost:7788`).
2.  Click the **AGOrg** chip in the top hero section next to the "Bus Status," or select the **AGOrg** tab from the main navigation (if visible) or via the dedicated side panel toggle.

### Step 2: Fill in Project Metadata
In the **Create AGOrg Project** card:

1.  **AGOrg Name**: Enter a human-readable name for your organization (e.g., `Arqon`, `MyProject`).
2.  **AGOrg Root Path**: Enter the absolute path to the directory containing your repositories.
    - *Tip*: Click the **Browse…** button to use your system's native folder picker.
    - > [!IMPORTANT]
      > **Flat Fleet Architecture**: Pilot operates on a "Flat Fleet" model. The **Root Path** should be the shared parent directory of your repositories (e.g., `~/Projects/arqon/`). 
      > Once Pilot finds a repository boundary (like `.git`), it will stop recursing. This preserves repository integrity and prevents unintentional nesting.
3.  **Parent AGOrg (Optional)**: If this organization is a child of another AGOrg (for nested hierarchies), enter the parent's UUID, Name, or Path. Creating a link here defines this AGOrg as a nested child in the organization tree. You can also use the **Browse…** button here.
4.  **Scan Depth**: Set how deep Pilot should search for repositories within the root path. The default is `4`.

### Step 3: Configure Automation Toggles
Before clicking create, review the execution flags:

- **Autoscan**: (Enabled by default) Automatically performs a directory discovery scan as soon as the project record is created. It looks for `.git` markers and `pyproject.toml` files.
- **Import Discovery**: (Enabled by default) Automatically registers any discovered repositories (AGOs) into the AGOrg database.
- **Set Default Scope**: (Enabled by default) Immediately switches the Pilot's active operational scope to this new AGOrg. This means subsequent actions in the Dashboard, Oracle, and Branch tabs will target this project.

### Step 4: Execute Creation
1.  Click the **Create Project** button.
2.  Watch the **Live Event Stream** at the bottom of the page. You will see events for:
    - `agorg_created`: Confirmation of the organization record.
    - `discovery_started`: Initialization of the directory scan.
    - `ago_registered`: Individual events for each repository found.
3.  The **Discovery and Tree** output panel on the right will update with a JSON preview of the discovered hierarchy.

---

## Part 2: Verifying the Scope

Once created, you can verify the active scope:
1.  Check the **Hero Section** at the top of the page. The AGOrg chip should now show your project name (e.g., `AGOrg: Arqon`) in green.
2.  Navigate to the **Dashboard**. All system status checks (Policy, Drift, etc.) will now be scoped to the repositories within your new AGOrg.

---

## Part 3: Understanding Discovery Results

After clicking **Create Project** with **Autoscan** enabled, Pilot performs a recursive search of your root directory.

### The Discovery Result Panel
The right-hand side of the AGOrg tab will populate with a JSON tree representing your organization's hierarchy:

- **Candidates**: A list of paths identified as either an **AGOrg** (containing children) or an **AGO** (a leaf repository).
- **Kind**: 
    - `agorg`: A container node, usually representing a project or a subgroup.
    - `ago`: A repository node where development work happens (contains `pyproject.toml` or `.git`).
- **Parent/Children Hints**: Metadata extracted from `pyproject.toml` under `[tool.arqon.relationships]`.

### Selective Registration
If you didn't check **Import Discovery** during creation, you can still register repos manually:
1.  Go to the **Discovery and Tree** section.
2.  Use the **Discovery Root** field to scan any directory.
3.  Specify the target AGOrg in **Import To (UUID or Name)**.
4.  Click **Discover** to see potential candidates before they are saved to your registry.

---

> [!TIP]
> Use the **Tree** button in the **Discovery and Tree** panel to view the current committed hierarchy for the active scope. This helps visualize how nested AGOrgs and AGOs are linked.
