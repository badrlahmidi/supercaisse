create or replace function public.is_admin_tenant()
returns boolean
language sql
security definer
stable
set search_path = public
as $$
  select coalesce((select role = 'admin_tenant' from public.profiles where id = auth.uid()), false)
$$;

revoke execute on function public.is_admin_tenant() from public;
revoke execute on function public.is_admin_tenant() from anon;
grant execute on function public.is_admin_tenant() to authenticated;

alter policy profiles_select on public.profiles
  using (
    id = auth.uid()
    or public.is_super_admin()
    or (
      tenant_id = public.current_tenant_id()
      and public.is_admin_tenant()
    )
  );

alter policy profiles_insert on public.profiles
  with check (
    public.is_super_admin()
    or (
      is_super_admin = false
      and tenant_id = public.current_tenant_id()
      and public.is_admin_tenant()
    )
  );

alter policy profiles_update on public.profiles
  using (
    id = auth.uid()
    or public.is_super_admin()
    or (
      tenant_id = public.current_tenant_id()
      and public.is_admin_tenant()
    )
  )
  with check (
    (id = auth.uid() and is_super_admin = false)
    or public.is_super_admin()
    or (
      is_super_admin = false
      and tenant_id = public.current_tenant_id()
      and public.is_admin_tenant()
    )
  );
