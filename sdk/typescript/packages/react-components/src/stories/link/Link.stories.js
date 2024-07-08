"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.InTextExample = exports.WithCustomChildren = exports.NoIcon = exports.Default = void 0;
var jsx_runtime_1 = require("react/jsx-runtime");
var material_1 = require("@mui/material");
var icons_material_1 = require("@mui/icons-material");
var link_1 = require("@lib/components/link");
exports.default = {
    title: 'Basics/Link',
    component: link_1.Link,
};
var Default = function () { return (0, jsx_runtime_1.jsx)(link_1.Link, { text: "link", href: "https://nymtech.net/", target: "_blank" }); };
exports.Default = Default;
var NoIcon = function () { return (0, jsx_runtime_1.jsx)(link_1.Link, { text: "link", href: "https://nymtech.net/", target: "_blank", noIcon: true }); };
exports.NoIcon = NoIcon;
var WithCustomChildren = function () { return ((0, jsx_runtime_1.jsx)(link_1.Link, { href: "https://nymtech.net/", target: "_blank", children: (0, jsx_runtime_1.jsx)(icons_material_1.Link, {}) })); };
exports.WithCustomChildren = WithCustomChildren;
var InTextExample = function () { return ((0, jsx_runtime_1.jsxs)(material_1.Typography, { children: ["You can find the Nym website ", (0, jsx_runtime_1.jsx)(link_1.Link, { href: "https://nymtech.net/", target: "_blank", text: "here" }), "."] })); };
exports.InTextExample = InTextExample;
