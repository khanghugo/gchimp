import type { NextConfig } from "next";
import path from 'path';

const firstCommitDate = new Date("Thu Apr 4 21:23:34 2024 -0400");
const currentDate = new Date();
const diffDate = Math.floor(((currentDate as any) - (firstCommitDate as any)) as number / (60 * 60 * 24 * 1000));

const generateCustomBuildId = () => {
  return `${diffDate} - ${currentDate.getFullYear()}-${currentDate.getMonth() + 1}-${currentDate.getDate()}, ${currentDate.getHours()}:${currentDate.getMinutes()}`
}

const nextConfig: NextConfig = {
  generateBuildId: generateCustomBuildId,

  turbopack: {
    resolveAlias: {
      'gchimp-web': "./gchimp-package/gchimp_web.js",
    },
  },

  // publicize buildId
  env: {
    GCHIMP_WEB_BUILD_ID: JSON.stringify(generateCustomBuildId()),
  },

  // FUCK OFF GIVE ME A COMMAND FOR THIS
  output: process.env.NEXTJS_EXPORT ? 'export' : 'standalone',
  distDir: "www",
};

export default nextConfig;
