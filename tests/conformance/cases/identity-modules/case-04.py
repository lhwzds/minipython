value = False
module_name = 'itertools'

module = __import__(module_name)
other = __import__(module_name)
log = CallLogger(); holder = Normal('holder', log, {value: module})
emit('identity', self=module is module, repeated=holder.value[value] is other, repeated_not=holder.value[value] is not other)
emit('module_operation', module=module_name, result=list(module.repeat(value, 2)))
