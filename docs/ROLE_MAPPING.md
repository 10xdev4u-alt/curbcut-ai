# Role and state mapping

> How the three platform accessibility APIs map onto the core `Role` and
> `ElementState`. The tables below mirror `ROLE_COMPAT` and `STATE_COMPAT`
> in `crates/curbcut-a11y/src/mapping.rs`; the tests fail if a row stops
> mapping to a concrete role.

## Raw role vocabularies

| Platform | Raw string source | Example |
| --- | --- | --- |
| macOS | `AXRole` attribute | `AXButton` |
| Windows | UIA `ControlType` name | `Button` |
| Linux | AT-SPI role name | `push-button` |

## Role compatibility table

### macOS

| Raw role | Core role |
| --- | --- |
| `AXButton` | `Button` |
| `AXLink` | `Link` |
| `AXTextField` | `TextField` |
| `AXTextArea` | `TextField` |
| `AXCheckBox` | `Checkbox` |
| `AXRadioButton` | `Radio` |
| `AXSwitch` | `Switch` |
| `AXMenuButton` | `Button` |
| `AXMenuItem` | `MenuItem` |
| `AXMenuBarItem` | `MenuItem` |
| `AXTabGroup` | `Tab` |
| `AXDialog` | `Dialog` |
| `AXSheet` | `Dialog` |
| `AXAlert` | `Alert` |
| `AXImage` | `Image` |
| `AXIcon` | `Icon` |
| `AXStaticText` | `StaticText` |
| `AXHeading` | `Heading` |
| `AXList` | `List` |
| `AXRow` | `ListItem` |
| `AXTable` | `Table` |

### Windows

| Raw role | Core role |
| --- | --- |
| `Button` | `Button` |
| `SplitButton` | `Button` |
| `Hyperlink` | `Link` |
| `Edit` | `TextField` |
| `ComboBox` | `TextField` |
| `CheckBox` | `Checkbox` |
| `RadioButton` | `Radio` |
| `ListItem` | `ListItem` |
| `DataItem` | `ListItem` |
| `TreeItem` | `ListItem` |
| `MenuItem` | `MenuItem` |
| `Tab` | `Tab` |
| `TabItem` | `Tab` |
| `Window` | `Dialog` |
| `Text` | `StaticText` |
| `Document` | `StaticText` |
| `TitleBar` | `StaticText` |
| `Image` | `Image` |
| `List` | `List` |
| `Menu` | `List` |
| `MenuBar` | `List` |
| `Tree` | `List` |
| `DataGrid` | `Table` |
| `Table` | `Table` |

### Linux

| Raw role | Core role |
| --- | --- |
| `push-button` | `Button` |
| `toggle-button` | `Switch` |
| `check-box` | `Checkbox` |
| `radio-button` | `Radio` |
| `combo-box` | `TextField` |
| `password-text` | `TextField` |
| `text-box` | `TextField` |
| `link` | `Link` |
| `list` | `List` |
| `list-item` | `ListItem` |
| `menu` | `List` |
| `menu-bar` | `List` |
| `menu-item` | `MenuItem` |
| `page-tab` | `Tab` |
| `page-tab-list` | `Tab` |
| `table` | `Table` |
| `table-cell` | `ListItem` |
| `tree` | `List` |
| `tree-table` | `Table` |
| `dialog` | `Dialog` |
| `alert` | `Alert` |
| `image` | `Image` |
| `icon` | `Icon` |
| `label` | `StaticText` |
| `heading` | `Heading` |
| `paragraph` | `StaticText` |
| `section` | `StaticText` |

## Unmapped roles

A raw role absent from the table becomes `Role::Unknown(raw)`. The raw
string is preserved, so an unmapped role is flagged in the provenance
panel rather than dropped. Container chrome with no core equivalent
(for example `AXScrollBar`, `ProgressBar`, `scroll-bar`) is deliberately
left unmapped.

## State mapping

`map_state` starts from `enabled: true` and applies the platform's state
names. A name the table does not know is ignored, so a new platform state
cannot corrupt an element. The adapter normalizes negative facts into the
table's negative rows: AT-SPI reports an absent `enabled` state as
`disabled`.

### macOS

| Raw state | Flag | Value |
| --- | --- | --- |
| `AXEnabled` | `Enabled` | `true` |
| `AXDisabled` | `Enabled` | `false` |
| `AXFocused` | `Focused` | `true` |
| `AXSelected` | `Selected` | `true` |
| `AXExpanded` | `Expanded` | `true` |
| `AXCollapsed` | `Expanded` | `false` |

### Windows

| Raw state | Flag | Value |
| --- | --- | --- |
| `IsEnabled` | `Enabled` | `true` |
| `IsDisabled` | `Enabled` | `false` |
| `HasKeyboardFocus` | `Focused` | `true` |
| `IsSelected` | `Selected` | `true` |
| `IsExpanded` | `Expanded` | `true` |
| `IsCollapsed` | `Expanded` | `false` |

### Linux

| Raw state | Flag | Value |
| --- | --- | --- |
| `enabled` | `Enabled` | `true` |
| `disabled` | `Enabled` | `false` |
| `focused` | `Focused` | `true` |
| `selected` | `Selected` | `true` |
| `expanded` | `Expanded` | `true` |
| `collapsed` | `Expanded` | `false` |

## Adding a platform role

1. Add a `RoleMapping` row to `ROLE_COMPAT`.
2. Add the raw string to the platform adapter's test fixture.
3. Regenerate this table.
