const CopyWebpackPlugin = require("copy-webpack-plugin");
const path = require("path");

module.exports = {
    performance: {
        hints: false,
        maxEntrypointSize: 512000,
        maxAssetSize: 512000,
    },
    entry: {
        bootstrap: "./bootstrap.js",
        worker: "./worker.js",
    },
    output: {
        path: path.resolve(__dirname, "dist"),
        filename: "[name].js",
    },
    mode: "development",
    // mode: 'production',
    plugins: [
        new CopyWebpackPlugin({
            patterns: [
                "index.html",
                {
                    from: "../pkg/*.(js|wasm)",
                    to: "[name][ext]",
                },
            ],
        }),
    ],
    devServer: {
        proxy: {
            '/api': {
                target: 'https://sandbox-nym-api1.nymtech.net/',
                secure: false,
            },
        },
        headers: {
            "Access-Control-Allow-Origin": "*",
            "Access-Control-Allow-Methods": "GET, POST, PUT, DELETE, PATCH, OPTIONS",
            "Access-Control-Allow-Headers":
                "X-Requested-With, content-type, Authorization",
        },
    },
    experiments: { syncWebAssembly: true },
};