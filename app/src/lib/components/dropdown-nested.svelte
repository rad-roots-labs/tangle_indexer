<script lang="ts">
    import { fmt_cl } from "@radroots/apps-lib";
    import { onDestroy, onMount, type Snippet } from "svelte";

    let {
        basis,
        primary,
        dropdown,
    }: {
        basis?: {
            classes_primary?: string;
            classes_dropdown?: string;
        };
        primary: Snippet;
        dropdown: Snippet;
    } = $props();

    let el_container: HTMLDetailsElement | null = $state(null);
    let el_dropdown: HTMLElement | null = $state(null);

    let shift_x = $state(0);

    $effect(() => {
        const rect_primary = el_container?.getBoundingClientRect();
        const rect_dropdown = el_dropdown?.getBoundingClientRect();
        if (rect_primary && rect_dropdown)
            shift_x = rect_dropdown.width - rect_primary.width;
    });

    const handle_click = (event: MouseEvent) => {
        if (!el_container || !el_dropdown) return;
        if (
            !el_container.contains(event.target as Node) ||
            el_dropdown.contains(event.target as Node)
        )
            el_container.open = false;
    };

    onMount(() => {
        document.addEventListener("click", handle_click);
    });

    onDestroy(() => {
        document.removeEventListener("click", handle_click);
    });
</script>

<details bind:this={el_container} class="dropdown dropdown-bottom">
    <summary class={`${fmt_cl(basis?.classes_primary)} flex`}>
        {@render primary()}
    </summary>
    <ul
        bind:this={el_dropdown}
        class={`dropdown-content flex flex-col pt-1`}
        style={`transform: translateX(-${shift_x}px);`}
    >
        <ul
            class={`z-50 ${fmt_cl(
                basis?.classes_dropdown || `menu min-w-52 p-2 bg-white shadow`,
            )} flex flex-col w-full justify-center items-center`}
        >
            {@render dropdown()}
        </ul>
    </ul>
</details>
