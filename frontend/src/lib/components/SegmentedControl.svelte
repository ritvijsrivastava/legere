<script lang="ts" generics="T extends string">
	let {
		name,
		options,
		value = $bindable()
	}: {
		name: string;
		options: { value: T; label: string }[];
		value: T;
	} = $props();
</script>

<div class="seg">
	{#each options as opt (opt.value)}
		<label class="seg-opt">
			<input
				type="radio"
				{name}
				value={opt.value}
				checked={value === opt.value}
				onchange={() => (value = opt.value)}
			/>
			<span>{opt.label}</span>
		</label>
	{/each}
</div>
