"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.Wordmark = exports.Icon = exports.Logo = void 0;
var jsx_runtime_1 = require("react/jsx-runtime");
var logo_1 = require("@lib/components/logo");
exports.default = {
    title: 'Branding/Nym Logo',
    component: logo_1.NymLogo,
};
var Logo = function () { return (0, jsx_runtime_1.jsx)(logo_1.NymLogo, { height: 250 }); };
exports.Logo = Logo;
var Icon = function () { return (0, jsx_runtime_1.jsx)(logo_1.NymIcon, { height: 250 }); };
exports.Icon = Icon;
var Wordmark = function () { return (0, jsx_runtime_1.jsx)(logo_1.NymWordmark, { height: 250 }); };
exports.Wordmark = Wordmark;
