import Link from "@docusaurus/Link";
import Logo from "../../static/img/logo.svg";
import { useEffect, useState } from "react";
import ExecutionEnvironment from "@docusaurus/ExecutionEnvironment";

const refreshDarkMode = () => {
  if (ExecutionEnvironment.canUseDOM) {
    document.documentElement.classList.toggle(
      "dark",
      localStorage.theme === "dark" ||
        (!("theme" in localStorage) &&
          window.matchMedia("(prefers-color-scheme: dark)").matches)
    );
  }
};

refreshDarkMode();

export default () => {
  const [theme, setTheme] = useState("light");

  useEffect(() => {
    setTimeout(() => {
      setTheme(
        localStorage.theme ??
          (window.matchMedia("(prefers-color-scheme: dark)").matches
            ? "dark"
            : "light")
      );
    }, 0);
  }, [ExecutionEnvironment.canUseDOM && localStorage.theme]);

  refreshDarkMode();

  const onClick = () => {
    if (theme === "light") {
      localStorage.theme = "dark";
      setTheme("dark");
      document.documentElement.classList.add("dark");
      document.documentElement.setAttribute("data-theme", "dark");
    }

    if (theme === "dark") {
      localStorage.theme = "light";
      setTheme("light");
      document.documentElement.classList.remove("dark");
      document.documentElement.setAttribute("data-theme", "light");
    }
  };

  const background = theme === "light" ? "e1cce2" : "8b6eb0";
  const offBackground = theme === "light" ? "d9c3de" : "9073b3";

  return (
    <div className="flex-grow relative h-full bg-quible-lightest dark:bg-quible-darkest">
      <div className="flex flex-col items-center">
        <div className="max-w-7xl self-center flex items-center justify-start max-auto px-5 w-full md:mb-20 leading-loose flex-wrap">
          <Link
            href="/"
            className="font-bold text-quible-darkest dark:text-quible-lightest p-5 hover:text-quible-medium dark:hover:text-quible-mild active:text-quible-mildest dark:active:text-quible-heavier hover:shadow-[inset_0_0_0_100px_#d9c3de] dark:hover:shadow-[inset_0_0_0_100px_#9073b3]"
          >
            Docs
          </Link>
          <a
            href="https://ts.docsend.com/view/5xyq5v7burxnmkib"
            className="font-bold text-quible-darkest dark:text-quible-lightest p-5 hover:text-quible-medium dark:hover:text-quible-mild active:text-quible-mildest dark:active:text-quible-heavier hover:shadow-[inset_0_0_0_100px_#d9c3de] dark:hover:shadow-[inset_0_0_0_100px_#9073b3]"
            target="_blank"
          >
            Whitepaper
          </a>
          <a
            href="https://twitter.com/QuibleNetwork"
            className="font-bold text-quible-darkest dark:text-quible-lightest p-5 hover:text-quible-medium dark:hover:text-quible-mild active:text-quible-mildest dark:active:text-quible-heavier box-content w-[20px] h-[20px] hover:shadow-[inset_0_0_0_100px_#d9c3de] dark:hover:shadow-[inset_0_0_0_100px_#9073b3]"
            target="_blank"
          >
            <svg width="20" height="20" fill="currentColor">
              <path d="M6.29 18.251c7.547 0 11.675-6.253 11.675-11.675 0-.178 0-.355-.012-.53A8.348 8.348 0 0020 3.92a8.19 8.19 0 01-2.357.646 4.118 4.118 0 001.804-2.27 8.224 8.224 0 01-2.605.996 4.107 4.107 0 00-6.993 3.743 11.65 11.65 0 01-8.457-4.287 4.106 4.106 0 001.27 5.477A4.073 4.073 0 01.8 7.713v.052a4.105 4.105 0 003.292 4.022 4.095 4.095 0 01-1.853.07 4.108 4.108 0 003.834 2.85A8.233 8.233 0 010 16.407a11.616 11.616 0 006.29 1.84"></path>
            </svg>
          </a>
          <a
            href="https://github.com/Quible-Network/quible-node"
            className="font-bold text-quible-darkest dark:text-quible-lightest p-5 hover:text-quible-medium dark:hover:text-quible-mild active:text-quible-mildest dark:active:text-quible-heavier flex items-center hover:shadow-[inset_0_0_0_100px_#d9c3de] dark:hover:shadow-[inset_0_0_0_100px_#9073b3]"
            target="_blank"
          >
            <svg width="20" height="20" viewBox="0 0 22 22" fill="currentColor">
              <path
                fillRule="evenodd"
                clipRule="evenodd"
                d="M12 2C6.477 2 2 6.463 2 11.97c0 4.404 2.865 8.14 6.839 9.458.5.092.682-.216.682-.48 0-.236-.008-.864-.013-1.695-2.782.602-3.369-1.337-3.369-1.337-.454-1.151-1.11-1.458-1.11-1.458-.908-.618.069-.606.069-.606 1.003.07 1.531 1.027 1.531 1.027.892 1.524 2.341 1.084 2.91.828.092-.643.35-1.083.636-1.332-2.22-.251-4.555-1.107-4.555-4.927 0-1.088.39-1.979 1.029-2.675-.103-.252-.446-1.266.098-2.638 0 0 .84-.268 2.75 1.022A9.606 9.606 0 0112 6.82c.85.004 1.705.114 2.504.336 1.909-1.29 2.747-1.022 2.747-1.022.546 1.372.202 2.386.1 2.638.64.696 1.028 1.587 1.028 2.675 0 3.83-2.339 4.673-4.566 4.92.359.307.678.915.678 1.846 0 1.332-.012 2.407-.012 2.734 0 .267.18.577.688.48C19.137 20.107 22 16.373 22 11.969 22 6.463 17.522 2 12 2z"
              ></path>
            </svg>
          </a>
          <div className="flex-grow"></div>
          <div
            onClick={onClick}
            className="cursor-pointer active:text-quible-deep text-quible-darkest m-5 inline-block h-[24px]"
          >
            {theme === "light" && (
              <svg
                key="light"
                xmlns="http://www.w3.org/2000/svg"
                fill="none"
                viewBox="0 0 24 24"
                strokeWidth="1.5"
                stroke="currentColor"
                className="size-6 w-[24px] h-[24px]"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  d="M12 3v2.25m6.364.386-1.591 1.591M21 12h-2.25m-.386 6.364-1.591-1.591M12 18.75V21m-4.773-4.227-1.591 1.591M5.25 12H3m4.227-4.773L5.636 5.636M15.75 12a3.75 3.75 0 1 1-7.5 0 3.75 3.75 0 0 1 7.5 0Z"
                />
              </svg>
            )}

            {theme === "dark" && (
              <svg
                key="dark"
                xmlns="http://www.w3.org/2000/svg"
                fill="none"
                viewBox="0 0 24 24"
                strokeWidth="1.5"
                stroke="currentColor"
                className="size-6 text-quible-lightest w-[24px] h-[24px]"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  d="M21.752 15.002A9.72 9.72 0 0 1 18 15.75c-5.385 0-9.75-4.365-9.75-9.75 0-1.33.266-2.597.748-3.752A9.753 9.753 0 0 0 3 11.25C3 16.635 7.365 21 12.75 21a9.753 9.753 0 0 0 9.002-5.998Z"
                />
              </svg>
            )}
          </div>
        </div>

        <div className="max-w-7xl w-full flex flex-wrap lg:flex-nowrap -mb-48 relative z-10 items-center">
          <div className="px-10 pb-20 sm:pb-0">
            <div className="flex self-start items-end">
              <Logo className="flex-grow-0 flex-shrink-0 w-[80px] h-[80px] md:w-[125px] md:h-[125px]" />
              <div className="text-quible-black dark:text-quible-white font-sans text-6xl md:text-8xl font-bold select-none leading-none">
                Quible
              </div>
            </div>
            <h1 className="text-quible-heaviest dark:text-quible-lighter text-4xl sm:text-3xl my-5 leading-normal">
              Multi-chain key management and access control
            </h1>
            <p className="text-quible-heaviest dark:text-quible-lighter text-2xl font-medium">
              We’re creating a new paradigm for protecting onchain assets. Stop distributing sensitive credentials to your workforce.
            </p>
            <a
              href="https://t.me/quiblealpha"
              target="_blank"
              className="inline-block my-5 !text-quible-lightest bg-quible-black dark:!text-quible-darker dark:bg-quible-white font-bold p-3 px-4 hover:text-quible-lighter hover:dark:text-quible-dark hover:bg-quible-darkest hover:dark:bg-quible-lightest active:text-quible-light active:dark:text-quible-deepest active:bg-quible-darker active:dark:bg-quible-lighter text-xl"
            >
              GET EARLY ACCESS
            </a>
          </div>
          <div className="self-stretch flex-grow">
            <div className="px-10 py-16 ml-10 xl:ml-20 mr-10 my-5 bg-quible-lighter text-quible-deepest dark:text-quible-lightest dark:bg-quible-deepest">
              <h1 className="text-4xl font-bold pb-5">Problem</h1>
              <div className="text-sm font-mono font-bold">
                Private key leaks are inevitable. Multi-sig wallets can’t protect everything. Countless organizations still rely on externally-owned accounts with single-sig protection.
              </div>
              <div className="my-16 h-px w-full bg-quible-deepest dark:bg-quible-lightest"></div>
              <h1 className="text-4xl font-bold pb-5">Solution</h1>
              <div className="text-sm font-mono font-bold">
                With certificate-based access control, Quible’s platform allows organizations to proactively protect themselves from attacks in ways that were not possible before.
              </div>
            </div>
          </div>
        </div>
        <div
          className="w-full bg-[center_top_1rem] relative"
          data-key={theme}
          key={theme}
          style={{
            backgroundImage: `
 url("data:image/svg+xml,%3Csvg width='84' height='88' viewBox='0 0 42 44' xmlns='http://www.w3.org/2000/svg'%3E%3Cg id='Page-1' fill-rule='evenodd'%3E%3Cg id='brick-wall' fill='%23${
   theme === "light" ? "ccb4d5" : "a98cc0"
 }' fill-opacity='0.4'%3E%3Cpath d='M0 0h42v44H0V0zm1 1h40v20H1V1zM0 23h20v20H0V23zm22 0h20v20H22V23z'/%3E%3C/g%3E%3C/g%3E%3C/svg%3E")
            `,
          }}
        >
          <div
            className="absolute inset-0 w-full h-full from-quible-lightest"
            style={{
              backgroundImage: `linear-gradient(to bottom, #${background}, transparent, transparent, #${background})`,
            }}
          ></div>
          <div className="flex justify-center">
            <div className="relative z-1 text-quible-black dark:text-quible-white max-w-5xl py-96">
              <h1 className="font-normal p-10 bg-quible-lightest dark:bg-quible-darkest text-3xl md:text-5xl leading-relaxed md:leading-snug mx-10">
                <span className="text-quible-deep dark:text-quible-mildest">
                  Single-signer EOAs are managing over
                </span>{" "}
                $26 billion 
                {" "}
                <span className="text-quible-deep dark:text-quible-mildest">
                  onchain. Let’s bring
                </span>{" "}
                  institutional security
                {" "}
                <span className="text-quible-deep dark:text-quible-mildest">
                  to the mainstream.
                </span>
              </h1>
            </div>
          </div>
        </div>
        <div className="relative self-stretch flex flex-col items-center bg-quible-lightest dark:bg-quible-darkest">
          <div className="absolute inset-0 bg-quible-lighter dark:bg-quible-darker" style={{
            maskRepeat: 'no-repeat',
            WebkitMaskRepeat: 'no-repeat',
            maskSize: 'contain',
            WebkitMaskSize: 'cover',
            maskPosition: 'center',
          maskImage: 'url(/img/nyc.svg)',
          WebkitMaskImage: 'url(/img/nyc.svg)'
        }}>
          </div>
          <div
            className="absolute inset-0 w-full h-full from-quible-lightest"
            style={{
              backgroundImage: `linear-gradient(to bottom, #${background}, transparent, transparent, transparent, transparent, #${background})`,
            }}
          ></div>
        <div className="w-full relative -mt-24 mb-24">
          <div className="w-full mt-32 mb-32 max-w-5xl mx-auto relative px-10">
            <div className="w-full h-px bg-quible-medium relative"></div>
          </div>
          <div className="w-full max-w-5xl mx-auto">
            <div className="text-quible-black dark:text-quible-white p-10 space-y-10 bg-quible-lightest dark:bg-quible-darkest">
              <h1 className="font-normal text-3xl md:text-5xl leading-relaxed md:leading-snug">
                Integrate in just a few lines of code
              </h1>
              <div>
                With Quible, you can implement safer security policies for you and your entire team.
              </div>
              <div className="border border-solid border-quible-medium w-full grid grid-cols-1 sm:grid-cols-2">
                <div className="w-full flex-grow flex-shrink-0 basis-1 border-b border-r-0 border-t-0 border-l-0 sm:border-b-0 sm:border-r border-solid border-quible-medium p-10 text-quible-black text-xl dark:text-quible-white">
                  <span className="font-mono text-quible-darker dark:text-quible-lighter pr-5">
                    01
                  </span>
                  Authenticate with certificates at scale
                </div>
                <div className="w-full flex-grow flex-shrink-0 basis-1 p-10 text-quible-black dark:text-quible-white text-xl">
                  <span className="font-mono text-quible-darker dark:text-quible-lighter pr-5">
                    02
                  </span>
                  Use fine-grained access policies
                </div>
              </div>
            </div>
          </div>
        </div>
        <div className="w-full relative -mt-24 mb-24">
          <div className="w-full mt-32 mb-32 max-w-5xl mx-auto relative px-10">
            <div className="w-full h-px bg-quible-medium relative"></div>
          </div>
          <div className="w-full max-w-5xl mx-auto">
            <div className="text-quible-black dark:text-quible-white p-10 space-y-10 bg-quible-lightest dark:bg-quible-darkest">
              <h1 className="font-normal text-3xl md:text-5xl leading-relaxed md:leading-snug">
                State of the art security
              </h1>
              <div>
                Get the best of both worlds. Leverage industry-leading security
                while supporting identity-aware protection.
              </div>
              <div className="grid grid-cols-1 sm:grid-cols-3 gap-5">
                <div className="w-full flex-grow flex-shrink-0 basis-1 p-10 text-quible-black text-xl dark:text-quible-white bg-quible-lighter dark:bg-quible-darker space-y-10">
                  <div className="text-center">Unprecented Design</div>
                  <div className="text-sm text-quible-darkest dark:text-quible-lighter">
                    Quible is implemented as a Layer 1 blockchain with custom
                    opcodes and architecture design, optimized for certificate
                    issuance and verification.
                  </div>
                </div>
                <div className="w-full flex-grow flex-shrink-0 basis-1 p-10 text-quible-black text-xl dark:text-quible-white bg-quible-lighter dark:bg-quible-darker space-y-10">
                  <div className="text-center">Unrivaled Performance</div>
                  <div className="text-sm text-quible-darkest dark:text-quible-lighter">
                    Quible is built from scratch in Rust, and attestations are
                    powered by low-latency threshold signatures.
                  </div>
                </div>
                <div className="w-full flex-grow flex-shrink-0 basis-1 p-10 text-quible-black text-xl dark:text-quible-white bg-quible-lighter dark:bg-quible-darker space-y-10">
                  <div className="text-center">Unlimited Scalability</div>
                  <div className="text-sm text-quible-darkest dark:text-quible-lighter">
                    Quible can support networks with hundreds of thousands or
                    even millions of identities, and has integration support
                    across hundreds of verticals.
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
        <div className="relative self-stretch flex flex-col items-center bg-quible-lightest dark:bg-quible-darkest">
          <div className="absolute inset-0 bg-quible-lighter dark:bg-quible-darker" style={{
            maskRepeat: 'no-repeat',
            WebkitMaskRepeat: 'no-repeat',
            maskSize: 'contain',
            WebkitMaskSize: 'cover',
            maskPosition: 'center',
          maskImage: 'url(/img/sf.svg)',
          WebkitMaskImage: 'url(/img/sf.svg)'
        }}>
          </div>
          <div
            className="absolute inset-0 w-full h-full from-quible-lightest"
            style={{
              backgroundImage: `linear-gradient(to bottom, #${background}, #${background}, transparent, transparent, #${background})`,
            }}
          ></div>
        <div className="w-full relative -mt-24 mb-24">
          <div className="w-full mt-32 mb-32 max-w-5xl mx-auto relative px-10">
            <div className="w-full h-px bg-quible-medium relative"></div>
          </div>
          <div className="w-full max-w-5xl mx-auto">
            <div className="text-quible-black dark:text-quible-white p-10 space-y-10 bg-quible-lightest dark:bg-quible-darkest">
              <h1 className="font-normal text-3xl md:text-5xl leading-relaxed md:leading-snug">
                Built for the future
              </h1>
              <div>
                Security infrastructure is a commodity, and Quible&rsquo;s
                marketplace lets you trade it like one.
              </div>
              <div className="grid grid-cols-1 sm:grid-cols-3 gap-5">
                <div className="w-full flex-grow flex-shrink-0 basis-1 p-10 text-quible-black text-xl dark:text-quible-white bg-quible-lighter dark:bg-quible-darker space-y-10">
                  <div className="text-center">IAM</div>
                  <div className="text-sm text-quible-darkest dark:text-quible-lighter">
                    Made possible by our certificate authority, applications pay
                    small fees to manage identities and utilize Quible&rsquo;s
                    open-source tooling to implement flexible access control.
                  </div>
                </div>
                <div className="w-full flex-grow flex-shrink-0 basis-1 p-10 text-quible-black text-xl dark:text-quible-white bg-quible-lighter dark:bg-quible-darker space-y-10">
                  <div className="text-center">Authentication</div>
                  <div className="text-sm text-quible-darkest dark:text-quible-lighter">
                    Quible&rsquo;s resource provisioning capabilities allows organizations to quickly create
                    authentication infrastructure for workforce and customer identity, including
                    OAuth 2.0 and MFA capabilities.
                  </div>
                </div>
                <div className="w-full flex-grow flex-shrink-0 basis-1 p-10 text-quible-black text-xl dark:text-quible-white bg-quible-lighter dark:bg-quible-darker space-y-10">
                  <div className="text-center">Network Protection</div>
                  <div className="text-sm text-quible-darkest dark:text-quible-lighter">
                    Provision flexible, identity-aware protection including bot mitigation and DDoS protection, for your Web3 cloud infrastructure.
                  </div>
                </div>
                {/* <div className="w-full flex-grow flex-shrink-0 basis-1 p-10 text-quible-black text-xl dark:text-quible-white bg-quible-lighter dark:bg-quible-darker space-y-10">
                  <div className="text-center">AI Agents</div>
                  <div className="text-sm text-quible-darkest dark:text-quible-lighter">
                    Secure large fleets of connected machines executing tasks
                  </div>
                </div>
                <div className="w-full flex-grow flex-shrink-0 basis-1 p-10 text-quible-black text-xl dark:text-quible-white bg-quible-lighter dark:bg-quible-darker space-y-10">
                  <div className="text-center">Machine Networks</div>
                  <div className="text-sm text-quible-darkest dark:text-quible-lighter">
                    Protect intra-network communication and network entry
                  </div>
                </div>
                <div className="w-full flex-grow flex-shrink-0 basis-1 p-10 text-quible-black text-xl dark:text-quible-white bg-quible-lighter dark:bg-quible-darker space-y-10">
                  <div className="text-center">Your Project</div>
                  <div className="text-sm text-quible-darkest dark:text-quible-lighter">
                    Quible can flexibly integrate anywhere, anytime
                  </div>
                </div> */}
              </div>
            </div>
          </div>
        </div>
        <div className="w-full relative">
          <div className="max-w-7xl mx-auto my-24 flex flex-col justify-center items-center gap-y-5">
            <div className="text-quible-black dark:text-quible-white text-4xl px-10">
              Frequently Asked Questions
            </div>

            <Link
              to="/"
              href="https://quible.network"
              className="flex gap-3 items-center justify-center hover:text-quible-medium dark:hover:text-quible-mild active:text-quible-mildest dark:active:text-quible-heavier hover:no-underline hover:opacity-90 active:opacity-50"
            ></Link>

            <Link
              to="/faq"
              className="inline-block my-5 !text-quible-lightest bg-quible-black dark:!text-quible-darker dark:bg-quible-white font-bold p-3 px-4 hover:text-quible-lighter hover:dark:text-quible-dark hover:bg-quible-darkest hover:dark:bg-quible-lightest active:text-quible-light active:dark:text-quible-deepest active:bg-quible-darker active:dark:bg-quible-lighter text-xl"
            >
              SEE FAQS
            </Link>
          </div>
        </div>
      </div>
        <div className="w-full relative">
          <div className="max-w-7xl mx-auto mb-10">
            <Link
              to="/"
              href="https://quible.network"
              className="flex gap-3 items-center justify-center hover:text-quible-medium dark:hover:text-quible-mild active:text-quible-mildest dark:active:text-quible-heavier hover:no-underline hover:opacity-90 active:opacity-50"
            >
              <Logo className="flex-grow-0 flex-shrink-0 w-[36px] h-[36px]" />
              <div className="text-quible-darkest dark:text-quible-white font-sans text-2xl font-bold select-none">
                Quible
              </div>
              <div className="text-quible-darkest dark:text-quible-lightest">
                © 2024 Quible Network. All rights reserved.
              </div>
            </Link>
          </div>
        </div>
      </div>
    </div>
  );
};
