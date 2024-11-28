import { themes as prismThemes } from "prism-react-renderer";
import type { Config } from "@docusaurus/types";
import type * as Preset from "@docusaurus/preset-classic";
import tailwindPlugin from "./plugins/tailwind-config.cjs";

// This runs in Node.js - Don't use client-side code here (browser APIs, JSX...)

const config: Config = {
  title: "Quible Docs",
  tagline: "The Decentralized Certificate Authority",
  favicon: "img/favicon.ico",

  // Set the production url of your site here
  url: "https://docs.quible.network",
  // Set the /<baseUrl>/ pathname under which your site is served
  // For GitHub pages deployment, it is often '/<projectName>/'
  baseUrl: "/",

  // GitHub pages deployment config.
  // If you aren't using GitHub pages, you don't need these.
  organizationName: "Quible-Network", // Usually your GitHub org/user name.
  projectName: "quible-node", // Usually your repo name.

  onBrokenLinks: "throw",
  onBrokenMarkdownLinks: "warn",

  // Even if you don't use internationalization, you can use this field to set
  // useful metadata like html lang. For example, if your site is Chinese, you
  // may want to replace "en" with "zh-Hans".
  i18n: {
    defaultLocale: "en",
    locales: ["en"],
  },

  plugins: [tailwindPlugin],

  presets: [
    [
      "classic",
      {
        docs: {
          routeBasePath: "/",
          sidebarPath: "./sidebars.ts",
          editUrl:
            "https://github.com/Quible-Network/quible-node/tree/main/docs.quible.network/docs",
        },
        theme: {
          customCss: "./src/css/custom.css",
        },
      } satisfies Preset.Options,
    ],
  ],

  themeConfig: {
    // Replace with your project's social card
    image: "img/docusaurus-social-card.jpg",
    navbar: {
      title: "Quible Docs",
      logo: {
        alt: "Quible Logo",
        src: "img/logo.svg",
      },
      items: [
        {
          type: "docSidebar",
          sidebarId: "tutorialSidebar",
          position: "left",
          label: "Docs",
        },
        {
          href: "https://github.com/Quible-Network",
          label: "GitHub",
          position: "right",
        },
      ],
    },
    footer: {
      style: "dark",
      links: [
        {
          title: "Community",
          items: [
            {
              label: "Early Access",
              href: "https://t.me/quiblealpha",
            },
            {
              label: "X",
              href: "https://x.com/QuibleNetwork",
            },
          ],
        },
        {
          title: "More",
          items: [
            {
              label: "GitHub",
              href: "https://github.com/Quible-Network/",
            },
          ],
        },
      ],
      copyright: `Copyright © ${new Date().getFullYear()} Quible Labs, Inc. Built with Docusaurus.`,
    },
    prism: {
      theme: prismThemes.github,
      darkTheme: prismThemes.dracula,
    },
    sidebar: {
      hideable: true
    }
  } satisfies Preset.ThemeConfig,

  headTags: [
    {
      tagName: "link",
      attributes: {
        rel: "icon",
        href: "/img/logo-16.png",
        sizes: "16x16",
      },
    },
    {
      tagName: "link",
      attributes: {
        rel: "icon",
        href: "/img/logo-32.png",
        sizes: "32x32",
      },
    },
    {
      tagName: "link",
      attributes: {
        rel: "icon",
        href: "/img/logo-64.png",
        sizes: "64x64",
      },
    },
    {
      tagName: "link",
      attributes: {
        rel: "icon",
        href: "/img/logo-128.png",
        sizes: "128x128",
      },
    },
    {
      tagName: "link",
      attributes: {
        rel: "icon",
        href: "/img/logo-250.png",
        sizes: "250x250",
      },
    },
    {
      tagName: "link",
      attributes: {
        rel: "icon",
        href: "/img/logo-256.png",
        sizes: "256x256",
      },
    },
    {
      tagName: "link",
      attributes: {
        rel: "icon",
        href: "/img/logo.png",
        sizes: "500x500",
      },
    },
    {
      tagName: "link",
      attributes: {
        rel: "icon",
        href: "/img/logo-512.png",
        sizes: "512x512",
      },
    },
    {
      tagName: "link",
      attributes: {
        rel: "apple-touch-icon",
        href: "/img/logo-16.png",
        sizes: "16x16",
      },
    },
    {
      tagName: "link",
      attributes: {
        rel: "apple-touch-icon",
        href: "/img/logo-32.png",
        sizes: "32x32",
      },
    },
    {
      tagName: "link",
      attributes: {
        rel: "apple-touch-icon",
        href: "/img/logo-64.png",
        sizes: "64x64",
      },
    },
    {
      tagName: "link",
      attributes: {
        rel: "apple-touch-icon",
        href: "/img/logo-128.png",
        sizes: "128x128",
      },
    },
    {
      tagName: "link",
      attributes: {
        rel: "apple-touch-icon",
        href: "/img/logo-250.png",
        sizes: "250x250",
      },
    },
    {
      tagName: "link",
      attributes: {
        rel: "apple-touch-icon",
        href: "/img/logo-256.png",
        sizes: "256x256",
      },
    },
    {
      tagName: "link",
      attributes: {
        rel: "apple-touch-icon",
        href: "/img/logo.png",
        sizes: "500x500",
      },
    },
    {
      tagName: "link",
      attributes: {
        rel: "apple-touch-icon",
        href: "/img/logo-512.png",
        sizes: "512x512",
      },
    },
  ],
};

export default config;
