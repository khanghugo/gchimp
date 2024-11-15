import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  /* config options here */
  webpack(config, options) {
    config.experiments = { ...config.experiments, asyncWebAssembly: true };

    return config;
  }
};

export default nextConfig;
