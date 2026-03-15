import type { NextConfig } from "next";

const allowTypeScriptBuildErrors = process.env.KYRO_ALLOW_TS_BUILD_ERRORS === "1";

const nextConfig: NextConfig = {
  output: "export",
  typescript: {
    ignoreBuildErrors: allowTypeScriptBuildErrors,
  },
  reactStrictMode: false,
  experimental: {
  },
  turbopack: {},
  // Optimize for production builds
  compress: true,
  poweredByHeader: false,
  // Static export requires unoptimized images (no server to resize)
  images: {
    unoptimized: true,
  },
  // Webpack configuration for Monaco Editor
  webpack: (config, { isServer }) => {
    if (!isServer) {
      config.resolve.fallback = {
        ...config.resolve.fallback,
        fs: false,
        net: false,
        tls: false,
      };
    }

    return config;
  },
};

export default nextConfig;
