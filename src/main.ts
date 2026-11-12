import "@fontsource-variable/bricolage-grotesque/wdth.css";
import "@fontsource-variable/hanken-grotesk/wght.css";
import "./app.css";
import { mount } from "svelte";
import App from "./App.svelte";

export default mount(App, { target: document.getElementById("app")! });
