create or replace function public.block_self_role_tenant_change()
returns trigger
language plpgsql
security definer
set search_path = public
as $$
begin
  if old.id = auth.uid() and not public.is_super_admin() then
    if new.role is distinct from old.role then
      raise exception 'Vous ne pouvez pas modifier votre propre role';
    end if;
    if new.tenant_id is distinct from old.tenant_id then
      raise exception 'Vous ne pouvez pas modifier votre propre tenant';
    end if;
  end if;
  return new;
end;
$$;

create trigger block_self_role_tenant_change_trigger
  before update on public.profiles
  for each row
  execute function public.block_self_role_tenant_change();

alter policy profiles_update on public.profiles
  using (
    id = auth.uid()
    or public.is_super_admin()
    or (
      tenant_id = public.current_tenant_id()
      and exists (
        select 1 from public.profiles p
        where p.id = auth.uid() and p.role = 'admin_tenant'
      )
    )
  )
  with check (
    (id = auth.uid() and is_super_admin = false)
    or public.is_super_admin()
    or (
      is_super_admin = false
      and tenant_id = public.current_tenant_id()
      and exists (
        select 1 from public.profiles p
        where p.id = auth.uid() and p.role = 'admin_tenant'
      )
    )
  );
