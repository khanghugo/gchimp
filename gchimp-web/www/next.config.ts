import type { NextConfig } from "next";

const firstCommitDate = new Date("Thu Apr 4 21:23:34 2024 -0400");
const currentDate = new Date();
const diffDate = Math.floor(((currentDate as any) - (firstCommitDate as any)) as number / (60 * 60 * 24 * 1000));

const nextConfig: NextConfig = {
  generateBuildId: async () => {
    // getMonth() starts from 0. Very nice. Fuck you.
    return `${currentDate.getFullYear()}-${currentDate.getMonth() + 1}-${currentDate.getDate()} (${diffDate})`;
  },

  webpack(config, { isServer, dev, buildId }) {
    config.experiments = {
      ...config.experiments,
      asyncWebAssembly: true,
      layers: true,
    };

    // config.output.webassemblyModuleFilename =
    //   isServer && !dev ? "../static/wasm/[id].wasm" : "static/wasm/[id].wasm";
    if (!dev && isServer) {
      config.output.webassemblyModuleFilename = "chunks/[id].wasm";
      config.plugins.push(new WasmChunksFixPlugin());
    }

    // publicize buildId
    process.env.GCHIMP_WEB_BUILD_ID = JSON.stringify(buildId);

    return config;
  },

  outputFileTracingIncludes: {
    '/api/**/*': ['./node_modules/**/*.wasm', './node_modules/**/*.proto'],
  },
  // FUCK OFF GIVE ME A COMMAND FOR THIS
  output: process.env.NEXTJS_EXPORT ? 'export' : 'standalone',
};

// HOLY FUCK. I AM GOING TO KILL WHOEVER GETS PAID TO DO THIS
// https://github.com/vercel/next.js/issues/29362#issuecomment-971377869
class WasmChunksFixPlugin {
  apply(compiler: any) {
    compiler.hooks.thisCompilation.tap("WasmChunksFixPlugin", (compilation: any) => {
      compilation.hooks.processAssets.tap(
        { name: "WasmChunksFixPlugin" },
        (assets: any) =>
          Object.entries(assets).forEach(([pathname, source]) => {
            if (!pathname.match(/\.wasm$/)) return;
            compilation.deleteAsset(pathname);

            const name = pathname.split("/")[1];
            const info = compilation.assetsInfo.get(pathname);
            compilation.emitAsset(name, source, info);
          })
      );
    });
  }
}

export default nextConfig;
