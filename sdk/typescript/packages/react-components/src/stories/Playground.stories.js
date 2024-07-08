"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.ThemeExplorer = exports.Palette = exports.AllControls = void 0;
var jsx_runtime_1 = require("react/jsx-runtime");
var material_1 = require("@mui/material");
var Playground_1 = require("../playground/Playground");
var theme_1 = require("../playground/theme");
var MUIThemeExplorer_1 = require("../playground/theme/MUIThemeExplorer");
exports.default = {
    title: 'Playground',
    component: Playground_1.Playground,
};
var AllControls = function () { return (0, jsx_runtime_1.jsx)(Playground_1.Playground, {}); };
exports.AllControls = AllControls;
var Palette = function () { return (0, jsx_runtime_1.jsx)(theme_1.PlaygroundPalette, {}); };
exports.Palette = Palette;
var ThemeExplorer = function () {
    var theme = (0, material_1.useTheme)();
    return (0, jsx_runtime_1.jsx)(MUIThemeExplorer_1.MUIThemeExplorer, { theme: theme });
};
exports.ThemeExplorer = ThemeExplorer;
